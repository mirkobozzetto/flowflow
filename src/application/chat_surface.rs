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
use crate::infrastructure::llm::{LlmClient, NotesTools};
use crate::infrastructure::persistence::installed_connector_repo::PinnedConnector;
use std::collections::{BTreeMap, BTreeSet};
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
// `armed` is the connector's device binding in gate format (v1 object / v2 array): when
// present it becomes the contract's bound_resource, so chat reads are bounded exactly
// like the chain path - one source of truth, one gate.
fn connector_entry(
    pin: &PinnedConnector,
    armed: Option<&serde_json::Value>,
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
        if !t.tool.starts_with(&conn.mcp_prefix) {
            eprintln!(
                "[chat_surface] {}: tool outside its owner prefix, dropped",
                pin.slug
            );
            return None;
        }
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
        bound_resource: armed.cloned(),
        column_roles: None,
        read_before_write: false,
        deny_destructive: true,
        on_multiple_match: None,
        limits: None,
    };
    Some((conn.mcp_prefix.clone(), gov, conn))
}

/// The mountable chat surface across every pinned connector. `armed` maps a connector SLUG
/// to its device binding in gate format; a connector with no entry mounts unbounded (the
/// current free-discovery behavior). Pure over its inputs, so the policy is unit-testable
/// without a registry or a network.
pub fn build_chat_surface(
    pins: &[PinnedConnector],
    armed: &BTreeMap<String, serde_json::Value>,
) -> ChatSurface {
    let contracts: Vec<_> = pins
        .iter()
        .filter_map(|pin| connector_entry(pin, armed.get(&pin.slug)))
        .collect();
    for (index, (prefix, _, manifest)) in contracts.iter().enumerate() {
        for (other_prefix, _, other_manifest) in &contracts[..index] {
            if prefix.starts_with(other_prefix)
                || other_prefix.starts_with(prefix)
                || manifest.tools.iter().any(|tool| {
                    other_manifest
                        .tools
                        .iter()
                        .any(|other| other.tool == tool.tool)
                })
            {
                return ChatSurface {
                    contracts: Vec::new(),
                    tool_names: BTreeSet::new(),
                };
            }
        }
    }
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
    notes_tools: NotesTools,
) -> Result<String, LlmError> {
    // No event channel = no card can render: notes-only surface, observe-only hook.
    let Some(tx) = status_tx.filter(|tx| !tx.is_closed()) else {
        let (dummy, _rx) = mpsc::unbounded_channel();
        return llm
            .run_agent(
                CHAT_MODEL,
                preamble,
                user_message,
                notes_tools.clone(),
                None,
                Vec::new(),
                ContractHook::new(dummy),
                0.3,
                4,
            )
            .await;
    };

    // The pool remains in this scope until the prompt finishes. Each mounted
    // tool and approval execution path receives its own connector's peer.
    let connection = connect_registry().await;
    let unavailable = connection.is_err();
    if let Err(reason) = &connection {
        eprintln!("[chat_surface] connectors unavailable: {reason}");
    }
    let connected = connection.ok().flatten();
    let (mounts, hook) = match connected.as_ref() {
        Some((_pool, surface, mounts)) => {
            let mounts = mounts.clone();
            let peers = mounts
                .iter()
                .flat_map(|(tools, peer)| {
                    tools
                        .iter()
                        .map(move |tool| (tool.name.to_string(), peer.clone()))
                })
                .collect();
            let hook =
                ContractHook::with_contracts(tx, surface.contracts.clone())
                    .with_peers(peers);
            (mounts, hook)
        }
        None => (Vec::new(), ContractHook::new(tx)),
    };
    let answer = llm
        .run_agent(
            CHAT_MODEL,
            preamble,
            user_message,
            notes_tools,
            None,
            mounts,
            hook,
            0.3,
            4,
        )
        .await?;
    if unavailable {
        Ok(format!("Connected services could not be loaded. This response used native tools only.\n\n{answer}"))
    } else {
        Ok(answer)
    }
}

type ChatConnection = (
    crate::infrastructure::mcp::McpPool,
    ChatSurface,
    Vec<(Vec<rmcp::model::Tool>, rmcp::service::ServerSink)>,
);

async fn connect_registry() -> Result<Option<ChatConnection>, String> {
    let db = crate::infrastructure::persistence::Database::open()
        .map_err(|error| error.to_string())?;
    let Some(backend) =
        crate::infrastructure::backend::BackendClient::from_db(&db)
    else {
        return Ok(None);
    };
    let pins = db.list_pinned_connectors();
    let armed = crate::application::connector_module::armed_bounds(&db);
    let surface = build_chat_surface(&pins, &armed);
    let entries: Vec<_> = pins
        .iter()
        .filter_map(|pin| {
            connector_entry(pin, armed.get(&pin.slug)).map(|(_, gov, _)| {
                (
                    pin.slug.clone(),
                    gov.tools
                        .into_iter()
                        .map(|tool| tool.tool)
                        .collect::<BTreeSet<_>>(),
                )
            })
        })
        .collect();
    if entries.is_empty() {
        return Ok(None);
    }
    if surface.tool_names.is_empty() {
        return Err("ambiguous pinned connector ownership".into());
    }
    let (slugs, allowed): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
    let pool = crate::infrastructure::mcp::McpPool::connect_chat(
        &db, &backend, &slugs,
    )
    .await
    .map_err(|error| error.to_string())?;
    let mounts = select_chat_mounts(pool.mounts(), &allowed)?;
    Ok(Some((pool, surface, mounts)))
}

/// Keep each descriptor attached to the peer that advertised it. Generic over
/// the peer so owner routing can be tested without a provider or LLM connection.
pub fn select_chat_mounts<P>(
    mounts: Vec<(Vec<rmcp::model::Tool>, P)>,
    allowed_by_owner: &[BTreeSet<String>],
) -> Result<Vec<(Vec<rmcp::model::Tool>, P)>, String> {
    if mounts.len() != allowed_by_owner.len() {
        return Err("connector mount count mismatch".into());
    }
    let mut seen = BTreeSet::new();
    let mut selected = Vec::new();
    for ((tools, peer), expected) in mounts.into_iter().zip(allowed_by_owner) {
        for tool in &tools {
            if !seen.insert(tool.name.to_string()) {
                return Err("ambiguous advertised tool ownership".into());
            }
        }
        if !expected
            .iter()
            .all(|name| tools.iter().any(|tool| tool.name.as_ref() == name))
        {
            return Err("connector is missing expected tools".into());
        }
        let tools: Vec<_> = tools
            .into_iter()
            .filter(|tool| expected.contains(tool.name.as_ref()))
            .collect();
        if !tools.is_empty() {
            selected.push((tools, peer));
        }
    }
    Ok(selected)
}
