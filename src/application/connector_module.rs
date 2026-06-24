// Runs one installed, pinned agent end to end: verifies + pins a signed package on first use, builds
// the agent from its manifest (governance, chain, preamble, the real bound_resource), dials the
// agent-scoped MCP route with x-agent-id, and enforces the governance gate on-device. The contract is
// no longer hardcoded here - it comes from the manifest the builder resolves.

use crate::application::agent_builder::{build_agent, BuiltAgent};
use crate::application::chain::{run_chain, ChainOutcome};
use crate::application::error::LlmError;
use crate::application::tools::ContractHook;
use crate::domain::agent_manifest::{
    digest_of_stored, parse_manifest, verify_package, ADMIN_PUBKEY,
};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::mcp::McpRegistry;
use crate::infrastructure::persistence::Database;
use tokio::sync::mpsc;

// `pub` so the contract-lock test verifies the shipped fixture against the pinned key and builds it.
// Must equal the manifest `id` AND the backend catalog id (seed_catalog: `agent-crm-sync`), since it
// travels as x-agent-id; an id the backend has not granted is rejected 403 at the proxy.
pub const FIXTURE_AGENT_ID: &str = "agent-crm-sync";
const SYNC_CHAIN_NAME: &str = "sync";

// The trigger UI's input for the chain run; the user's goal, not part of the pinned contract.
const SYNC_GOAL: &str =
    "List my spreadsheets, open the most relevant one, and report what it contains.";

// A signed agent package shipped with the app to prove the install -> verify -> pin -> build -> run
// path on device without backend install plumbing. The signature is over the canonical digest of the
// manifest, made offline with the dev admin key whose public half is pinned in `ADMIN_PUBKEY`.
// ponytail: `bound_resource.spreadsheet_id` is a placeholder; to bind a real sheet, edit the manifest,
// rerun `cargo test -p flowflow --test agent_manifest_test gen_fixture -- --ignored --nocapture`, and
// paste the new content_digest + signature below. Until then off-bound reads/writes are refused, so
// only the read-only `find` state runs live - exactly the M1.14 safe state.
pub const FIXTURE_PACKAGE: &str = r#"{
  "manifest": {
    "schema_version": "1",
    "id": "agent-crm-sync",
    "version": "1.0.0",
    "name": "CRM Sync",
    "description": "Use when the user wants to read or update their client/prospect spreadsheet. Do NOT use for general questions or the calendar.",
    "author": "flowflow-admin",
    "alias": "synchro-clients",
    "model": "gpt-4o",
    "temperature": 0.1,
    "required_connectors": [
      { "type": "tabular_store", "capabilities": ["search", "read", "update"] }
    ],
    "system_prompt": "You keep the user's client spreadsheet tidy. Read before you write, and act only on the bound sheet.",
    "governance": {
      "tools": [
        { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" },
        { "tool": "google_sheets_get_spreadsheet",   "mode": "read_only" },
        { "tool": "google_sheets_write_to_cell",     "mode": "read_write" }
      ],
      "bound_resource": { "spreadsheet_id": "bound-at-install" },
      "read_before_write": true,
      "deny_destructive": true,
      "limits": { "max_steps": 6, "max_tool_calls": 30 }
    },
    "orchestration": {
      "chains": {
        "sync": {
          "initial": "find",
          "states": {
            "find":   { "allowed_tools": ["google_sheets_list_spreadsheets"], "on_done": "read" },
            "read":   { "allowed_tools": ["google_sheets_get_spreadsheet"], "on_done": "act" },
            "act":    { "allowed_tools": ["google_sheets_write_to_cell"], "guard": "read_before_write", "on_done": "answer" },
            "answer": { "terminal": true }
          }
        }
      }
    }
  },
  "content_digest": "sha256:b6a684ce863b9b8cb8c7b941fe55be911fb07198e52d1efac2bf7c45c975fd20",
  "signature": "ed25519:2TosOZou18cjSkc+jC+7NQA0ewfv1F6JH/HgzYr9XnWYXmslDwASg6jkPq0Gx4d4aUx+6yOSXQ7lMEsgiv+JCA==",
  "signer_key_id": "dev-admin",
  "status": "published"
}"#;

/// Fire `google_sheets_list_spreadsheets` through the installed agent's gated path and return the
/// rendered answer. Errors surface as a display string for the trigger UI.
pub async fn list_spreadsheets(db: &Database) -> Result<String, String> {
    let built = load_built(db)?;
    let preamble = format!(
        "{}\n\nFor this test, call only google_sheets_list_spreadsheets, then reply with the list. \
         Call no other tool.",
        built.preamble
    );
    run_module(db, &built, &preamble, "List my spreadsheets.")
        .await
        .map_err(|e| e.to_string())
}

/// Drive the installed agent's pinned `sync` chain through the FSM runtime and return a per-state
/// trace for the trigger UI.
pub async fn run_sync_chain(db: &Database) -> Result<String, String> {
    let built = load_built(db)?;
    let chain = built
        .chains
        .get(SYNC_CHAIN_NAME)
        .ok_or_else(|| format!("manifest has no `{SYNC_CHAIN_NAME}` chain"))?;
    let outcome = run_chain(
        db,
        chain,
        built.governance.clone(),
        built.connector.clone(),
        &built.slug,
        &built.agent_id,
        SYNC_GOAL,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(format_outcome(&outcome))
}

// Install (verify + pin) the fixture on first use, then load the pinned row, re-check its integrity
// against the pinned digest, and build the runnable agent from its manifest.
fn load_built(db: &Database) -> Result<BuiltAgent, String> {
    ensure_fixture_installed(db)?;
    let installed = db
        .get_installed_agent(FIXTURE_AGENT_ID)
        .ok_or("agent not installed")?;

    let recomputed = digest_of_stored(&installed.manifest_json)
        .map_err(|e| e.to_string())?;
    if recomputed != installed.content_digest {
        return Err(format!(
            "pinned digest mismatch for `{FIXTURE_AGENT_ID}`: stored manifest no longer hashes to its pin"
        ));
    }

    let manifest =
        parse_manifest(&installed.manifest_json).map_err(|e| e.to_string())?;
    build_agent(&manifest)
}

fn ensure_fixture_installed(db: &Database) -> Result<(), String> {
    // Verify the shipped fixture every time (cheap), and repin whenever the device has no row or a row
    // at a different digest, so a newer shipped manifest replaces the stale pin instead of running it.
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY)
        .map_err(|e| format!("verify fixture agent: {e}"))?;
    match db.get_installed_agent(FIXTURE_AGENT_ID) {
        Some(existing)
            if existing.content_digest == verified.content_digest =>
        {
            Ok(())
        }
        _ => db.install_agent(&verified),
    }
}

// Connect the agent-scoped MCP, enforce the device gate via the built contract, and run one prompt.
// `reg` is held in scope for the whole prompt so the tools' server sink stays valid.
async fn run_module(
    db: &Database,
    built: &BuiltAgent,
    preamble: &str,
    user_message: &str,
) -> Result<String, LlmError> {
    let llm = LlmClient::from_db(db)?;
    let backend = BackendClient::from_db(db).ok_or_else(|| {
        LlmError::NotConfigured("no backend configured".into())
    })?;

    let reg =
        McpRegistry::connect_agent(db, &backend, &built.slug, &built.agent_id)
            .await
            .map_err(|e| LlmError::Completion(format!("mcp connect: {e}")))?;
    if reg.is_empty() {
        return Err(LlmError::Completion(
            "agent-scoped MCP exposed no tools".into(),
        ));
    }

    let (tx, _rx) = mpsc::unbounded_channel();
    let hook = ContractHook::with_contract(
        tx,
        built.governance.clone(),
        built.connector.clone(),
    );
    llm.run_mcp_agent(preamble, user_message, &reg, hook).await
}

// Render the ground-truth tool log first (what each state actually called and got back), then the
// model's rendered reply. The tool lines are the source of truth; the reply narrates over them.
fn format_outcome(outcome: &ChainOutcome) -> String {
    let mut out = String::new();
    for step in &outcome.trace {
        out.push_str(&format!("[{}]\n", step.state));
        for line in &step.tools {
            out.push_str(&format!("  - {line}\n"));
        }
        out.push_str(&format!("  {}\n", step.outcome));
    }
    out.trim_end().to_string()
}
