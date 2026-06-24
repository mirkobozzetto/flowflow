// M1.13 atomic module (issue #12): run ONE pinned connector module end to end through the
// agent-scoped, gate-enforced path. The device builds a minimal agent from a pinned contract,
// dials `/v1/connectors/{slug}/mcp` with `x-agent-id`, and the governance gate runs on BOTH sides
// (this device hook before the call leaves, the backend proxy before it reaches the connector).
//
// Manifest install/pin (M1.15) is not built yet, so the contract is HARDCODED here: the Sheets
// connector manifest plus a single-tool read_only governance. When pinning lands, only the source
// of `gov`/`conn`/`AGENT_ID`/`SLUG` changes; the orchestration below stays.

use crate::application::error::LlmError;
use crate::application::tools::ContractHook;
use crate::domain::governance::{parse_connector_manifest, parse_governance};
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

    // `with_contract` always needs a status channel; keep the receiver alive so the hook's sends
    // land in the buffer even though this path surfaces no UI tool events.
    let (tx, _rx) = mpsc::unbounded_channel();
    let hook = ContractHook::with_contract(tx, gov, conn);

    llm.run_mcp_agent(preamble, user_message, &reg, hook).await
}
