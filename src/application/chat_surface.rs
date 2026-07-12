// The chat/generic-action tool surface: which connector tools an open-ended agent run may
// see, under which contract. Chat context carries untrusted content (web snippets,
// attachment text), so every mounted connector tool is governed: reads pass, writes hold
// for the user's approval card. Tools without a pinned manifest, row tools (need an armed
// schema), and destructive tools never mount here - the chain path owns those.

use crate::application::constants::CHAT_MODEL;
use crate::application::error::LlmError;
use crate::application::tools::{ContractHook, ToolEvent, NOTES_TOOL_NAMES};
use crate::domain::governance::{
    is_row_tool, parse_connector_manifest, Action, Approval, ConnectorManifest,
    Governance, Mode, Risk, ToolPolicy,
};
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::installed_connector_repo::PinnedConnector;
use std::collections::BTreeSet;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct ChatSurface {
    // One (mcp_prefix, governance, manifest) entry per mountable connector.
    pub contracts: Vec<(String, Governance, ConnectorManifest)>,
    // Exact tool names the chat agent may be mounted with.
    pub tool_names: BTreeSet<String>,
}

// The chat grant per manifest action. None = the tool never mounts in chat: upsert needs
// key_columns chat cannot supply, clear/delete are destructive (no mode grants them).
fn mode_for(action: Action) -> Option<Mode> {
    match action {
        Action::Search | Action::Read => Some(Mode::ReadOnly),
        Action::Append => Some(Mode::AppendOnly),
        Action::Update | Action::Create => Some(Mode::ReadWrite),
        Action::Upsert | Action::Clear | Action::Delete => None,
    }
}

// Derive one connector's chat contract from its pinned manifest. None = the connector
// fails the surface build and mounts nothing (fail closed): unparseable manifest, empty
// prefix (would catch-all foreign tools), a notes-tool name collision, or an incoherent
// classification (a write-classified action marked risk read_only cannot be trusted).
fn connector_entry(
    pin: &PinnedConnector,
) -> Option<(String, Governance, ConnectorManifest)> {
    let conn = match parse_connector_manifest(&pin.manifest_json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[chat_surface] {}: manifest unparseable: {e}", pin.slug);
            return None;
        }
    };
    if conn.mcp_prefix.is_empty() {
        eprintln!("[chat_surface] {}: empty mcp_prefix, dropped", pin.slug);
        return None;
    }
    for t in &conn.tools {
        if NOTES_TOOL_NAMES.contains(&t.tool.as_str()) {
            eprintln!(
                "[chat_surface] {}: tool `{}` collides with a reserved notes tool, connector dropped",
                pin.slug, t.tool
            );
            return None;
        }
        if t.action.is_write() && t.risk == Risk::ReadOnly {
            eprintln!(
                "[chat_surface] {}: `{}` is a write action marked read_only, connector dropped",
                pin.slug, t.tool
            );
            return None;
        }
    }
    let tools: Vec<ToolPolicy> = conn
        .tools
        .iter()
        .filter(|t| !is_row_tool(&t.tool) && t.risk != Risk::Destructive)
        .filter_map(|t| {
            mode_for(t.action).map(|mode| ToolPolicy {
                tool: t.tool.clone(),
                mode,
                approval: Approval::RequireApproval,
                key_columns: None,
                max_calls_per_run: None,
            })
        })
        .collect();
    if tools.is_empty() {
        return None;
    }
    let gov = Governance {
        tools,
        bound_resource: None,
        column_roles: None,
        read_before_write: false,
        deny_destructive: true,
        on_multiple_match: None,
        limits: None,
    };
    Some((conn.mcp_prefix.clone(), gov, conn))
}

/// The mountable chat surface across every pinned connector. Pure over its inputs, so the
/// policy is unit-testable without a registry or a network.
pub fn build_chat_surface(pins: &[PinnedConnector]) -> ChatSurface {
    let contracts: Vec<_> = pins.iter().filter_map(connector_entry).collect();
    let tool_names = contracts
        .iter()
        .flat_map(|(_, gov, _)| gov.tools.iter().map(|tp| tp.tool.clone()))
        .collect();
    ChatSurface {
        contracts,
        tool_names,
    }
}

/// Run the chat/generic-action agent: notes tools always, governed connector tools when a
/// backend is connected AND a live event channel exists to carry approval cards. This is
/// the ONLY place an open-ended agent run mounts connector tools.
pub async fn prompt_chat_agent(
    llm: Arc<LlmClient>,
    preamble: &str,
    user_message: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
) -> Result<String, LlmError> {
    // No event channel = no card can render: notes-only surface, observe-only hook.
    let Some(tx) = status_tx else {
        let (dummy, _rx) = mpsc::unbounded_channel();
        return llm
            .run_agent(
                CHAT_MODEL,
                preamble,
                user_message,
                true,
                None,
                ContractHook::new(dummy),
                0.3,
                4,
            )
            .await;
    };

    // Connector tools are dark unless a backend is configured AND reachable. The registry
    // is held in scope for the whole prompt so the tools' server sink stays valid.
    let reg = connect_registry().await;
    let surface = reg.as_ref().and_then(|reg| {
        let db = crate::infrastructure::persistence::Database::open().ok()?;
        let surface = build_chat_surface(&db.list_pinned_connectors());
        let (mounted, dropped): (Vec<_>, Vec<_>) = reg
            .tools()
            .into_iter()
            .partition(|t| surface.tool_names.contains(t.name.as_ref()));
        if !dropped.is_empty() {
            let names: Vec<_> =
                dropped.iter().map(|t| t.name.as_ref()).collect();
            eprintln!(
                "[chat_surface] not mounted in chat (no manifest backing or chain-only): {}",
                names.join(", ")
            );
        }
        if mounted.is_empty() {
            return None;
        }
        Some((mounted, surface.contracts, reg.peer()))
    });

    match surface {
        Some((mounted, contracts, peer)) => {
            let hook = ContractHook::with_contracts(tx, contracts)
                .with_peer(peer.clone());
            llm.run_agent(
                CHAT_MODEL,
                preamble,
                user_message,
                true,
                Some((mounted, peer)),
                hook,
                0.3,
                4,
            )
            .await
        }
        None => {
            llm.run_agent(
                CHAT_MODEL,
                preamble,
                user_message,
                true,
                None,
                ContractHook::new(tx),
                0.3,
                4,
            )
            .await
        }
    }
}

// Connect to the backend MCP proxy if one is configured. None (notes-only agent) when no
// backend is set or the connect fails.
async fn connect_registry() -> Option<crate::infrastructure::mcp::McpRegistry> {
    let db = crate::infrastructure::persistence::Database::open().ok()?;
    let backend = crate::infrastructure::backend::BackendClient::from_db(&db)?;
    match crate::infrastructure::mcp::McpRegistry::connect(&db, &backend).await
    {
        Ok(reg) => Some(reg),
        Err(e) => {
            eprintln!("MCP connect failed, notes tools only: {e}");
            None
        }
    }
}
