// Runs one pinned connector module end to end: builds a minimal agent, dials the agent-scoped
// MCP route with x-agent-id, and enforces the governance gate on-device. The contract is
// hardcoded until manifest pinning exists.

use crate::application::chain::{run_chain, ChainOutcome};
use crate::application::error::LlmError;
use crate::application::tools::ContractHook;
use crate::domain::governance::{parse_connector_manifest, parse_governance};
use crate::domain::orchestration::parse_chain;
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::mcp::McpRegistry;
use crate::infrastructure::persistence::Database;
use tokio::sync::mpsc;

// The entitled agent driving the call (sent as `x-agent-id`) and the connector slug (the URL
// segment). Hardcoded for the atomic proof; sourced from a pinned manifest once M1.15 ships.
const AGENT_ID: &str = "agent-crm-sync";
const SLUG: &str = "google";

// `pub` so the drift test asserts the gate Allows `list_spreadsheets` and Denies any other tool.
// The manifest mirrors `marketplace-flowflow/connectors/google-sheets.json` (the connector's full
// surface); the governance allows exactly one read_only tool.
pub const SHEETS_CONNECTOR_MANIFEST: &str = r#"{
  "connector": "google-sheets",
  "type": "tabular_store",
  "server": "ghcr.io/klavis-ai/google-sheets-mcp-server",
  "mcp_prefix": "google_sheets_",
  "provides": ["search", "read", "create", "update"],
  "tools": [
    { "tool": "google_sheets_list_spreadsheets",  "resource": "spreadsheet", "action": "search", "risk": "read_only" },
    { "tool": "google_sheets_get_spreadsheet",    "resource": "spreadsheet", "action": "read",   "risk": "read_only" },
    { "tool": "google_sheets_list_sheets",        "resource": "sheet",       "action": "read",   "risk": "read_only" },
    { "tool": "google_sheets_create_spreadsheet", "resource": "spreadsheet", "action": "create", "risk": "read_write" },
    { "tool": "google_sheets_create_sheets",      "resource": "sheet",       "action": "create", "risk": "read_write" },
    { "tool": "google_sheets_write_to_cell",      "resource": "cell",        "action": "update", "risk": "read_write" }
  ]
}"#;

pub const SHEETS_LIST_ONLY_GOVERNANCE: &str = r#"{"tools":[{"tool":"google_sheets_list_spreadsheets","mode":"read_only"}]}"#;

// A `list_spreadsheets` call is `action=search`, so it bypasses bound_resource and
// read_before_write: the cleanest end-to-end proof of the gated path.
const LIST_PREAMBLE: &str = "You are a connector test agent. Call \
    google_sheets_list_spreadsheets to list the user's spreadsheets, then reply with the \
    list. Call no other tool.";

/// Fire `google_sheets_list_spreadsheets` through the agent-scoped, gate-enforced path and return
/// the agent's rendered answer. Errors are surfaced as a display string for the trigger UI.
pub async fn list_spreadsheets(db: &Database) -> Result<String, String> {
    run_pinned_module(db, LIST_PREAMBLE, "List my spreadsheets.")
        .await
        .map_err(|e| e.to_string())
}

// Build the minimal agent from the pinned contract, connect the agent-scoped MCP, enforce the
// device gate via `ContractHook::with_contract`, and run one prompt. `reg` is held in scope for
// the whole prompt so the tools' server sink stays valid.
async fn run_pinned_module(
    db: &Database,
    preamble: &str,
    user_message: &str,
) -> Result<String, LlmError> {
    let llm = LlmClient::from_db(db)?;
    let backend = BackendClient::from_db(db).ok_or_else(|| {
        LlmError::NotConfigured("no backend configured".into())
    })?;

    let reg = McpRegistry::connect_agent(db, &backend, SLUG, AGENT_ID)
        .await
        .map_err(|e| LlmError::Completion(format!("mcp connect: {e}")))?;
    if reg.is_empty() {
        return Err(LlmError::Completion(
            "agent-scoped MCP exposed no tools".into(),
        ));
    }

    let conn =
        parse_connector_manifest(SHEETS_CONNECTOR_MANIFEST).map_err(|e| {
            LlmError::Completion(format!("connector manifest: {e}"))
        })?;
    let gov = parse_governance(SHEETS_LIST_ONLY_GOVERNANCE)
        .map_err(|e| LlmError::Completion(format!("governance: {e}")))?;

    // `with_contract` always needs a status channel; this path surfaces no UI tool events, so the
    // receiver is a discard sink (the hook's status sends are fire-and-forget).
    let (tx, _rx) = mpsc::unbounded_channel();
    let hook = ContractHook::with_contract(tx, gov, conn);

    llm.run_mcp_agent(preamble, user_message, &reg, hook).await
}

// The first chain: find -> read -> act -> answer. Each non-terminal state exposes exactly one Sheets tool;
// `act` carries the read_before_write guard; `answer` is terminal and composes the reply. The schema is the
// contract whether the FSM is hand-rolled or adopts a library runtime later.
pub const SHEETS_SYNC_CHAIN: &str = r#"{
  "initial": "find",
  "states": {
    "find":   { "allowed_tools": ["google_sheets_list_spreadsheets"], "on_done": "read" },
    "read":   { "allowed_tools": ["google_sheets_get_spreadsheet"],   "on_done": "act" },
    "act":    { "allowed_tools": ["google_sheets_write_to_cell"], "guard": "read_before_write", "on_done": "answer" },
    "answer": { "terminal": true }
  }
}"#;

// Governance for the full chain: search + read run free, the single-cell write is read_write behind the
// read_before_write floor, destructive tools refused. The write is pinned to a target so the floor means
// "read the resource you write", not merely "some read happened".
// ponytail: the bound spreadsheet id is a placeholder until manifest pinning supplies the real one; until
// then only the read-only `find` runs live (the backend serves no write), and off-bound writes are refused.
pub const SHEETS_SYNC_GOVERNANCE: &str = r#"{
  "tools": [
    { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" },
    { "tool": "google_sheets_get_spreadsheet",    "mode": "read_only" },
    { "tool": "google_sheets_write_to_cell",      "mode": "read_write" }
  ],
  "bound_resource": { "spreadsheet_id": "bound-at-install" },
  "read_before_write": true,
  "deny_destructive": true,
  "limits": { "max_steps": 6, "max_tool_calls": 30 }
}"#;

const SYNC_GOAL: &str =
    "List my spreadsheets, open the most relevant one, and report what it contains.";

/// Drive the pinned Sheets chain through the FSM runtime and return a per-state trace for the trigger UI.
pub async fn run_sync_chain(db: &Database) -> Result<String, String> {
    let chain = parse_chain(SHEETS_SYNC_CHAIN)
        .map_err(|e| format!("chain manifest: {e}"))?;
    let conn = parse_connector_manifest(SHEETS_CONNECTOR_MANIFEST)
        .map_err(|e| format!("connector manifest: {e}"))?;
    let gov = parse_governance(SHEETS_SYNC_GOVERNANCE)
        .map_err(|e| format!("governance: {e}"))?;

    let outcome = run_chain(db, &chain, gov, conn, SLUG, AGENT_ID, SYNC_GOAL)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format_outcome(&outcome))
}

// Render the ground-truth tool log first (what each state actually called and got back), then the model's
// rendered reply. The tool lines are the source of truth; the reply is the model's narration over them.
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
