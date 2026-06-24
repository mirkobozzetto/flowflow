// Builds a runnable agent from a pinned manifest (RFC 0010 docs/protocol/09): it resolves the
// capability-style `required_connectors` to an armed connector, validates the manifest's governance
// against that connector and its chains for soundness, and yields the model, the governed tool
// subset, the real `bound_resource`, the chains, and a preamble (advisory `system_prompt` + a
// generated capability summary so the model is aware of what it can do). This replaces the contract
// that connector_module used to hardcode.

use std::collections::BTreeMap;

use crate::domain::agent_manifest::AgentManifest;
use crate::domain::governance::{
    parse_connector_manifest, validate_governance, ConnectorManifest,
    Governance,
};
use crate::domain::orchestration::Chain;

// The connector data manifest (the connector's full MCP surface mapped onto resource/action/risk).
// Mirrors marketplace-flowflow/connectors/google-sheets.json. `pub` so the builder test asserts the
// gate Allows the governed read-only tool and Denies anything ungoverned.
pub const SHEETS_CONNECTOR_MANIFEST_JSON: &str = r#"{
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

// Capability-type -> (armed connector slug, its data manifest). One connector today; a real registry
// or a backend fetch replaces this match when a second connector ships.
pub fn connector_for_type(
    connector_type: &str,
) -> Option<(&'static str, &'static str)> {
    match connector_type {
        "tabular_store" => Some(("google", SHEETS_CONNECTOR_MANIFEST_JSON)),
        _ => None,
    }
}

// The resolved, runnable contract handed to the chain runtime / module path. `model` is surfaced for
// when per-agent model selection lands; the run still uses the provider-configured LlmClient today.
#[derive(Debug)]
pub struct BuiltAgent {
    pub agent_id: String,
    pub slug: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub preamble: String,
    pub governance: Governance,
    pub connector: ConnectorManifest,
    pub chains: BTreeMap<String, Chain>,
}

/// Resolve + validate a manifest into a runnable agent. Fails closed: an unarmed connector type,
/// governance that does not validate against the connector, or an unsound chain all stop the build.
pub fn build_agent(manifest: &AgentManifest) -> Result<BuiltAgent, String> {
    let req = manifest
        .required_connectors
        .first()
        .ok_or("manifest declares no required_connectors")?;
    let (slug, conn_json) = connector_for_type(&req.connector_type)
        .ok_or_else(|| {
            format!("no connector armed for type `{}`", req.connector_type)
        })?;
    let connector = parse_connector_manifest(conn_json)
        .map_err(|e| format!("connector manifest: {e}"))?;

    validate_governance(&manifest.governance, &connector).map_err(|errs| {
        let joined = errs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        format!("governance invalid: {joined}")
    })?;

    for (name, chain) in &manifest.orchestration.chains {
        chain
            .validate()
            .map_err(|e| format!("chain `{name}`: {e}"))?;
    }

    Ok(BuiltAgent {
        agent_id: manifest.id.clone(),
        slug: slug.to_string(),
        model: manifest.model.clone(),
        temperature: manifest.temperature,
        preamble: preamble_for(manifest, &connector),
        governance: manifest.governance.clone(),
        connector,
        chains: manifest.orchestration.chains.clone(),
    })
}

/// Merge an arm-time binding over the manifest's `bound_resource`. The per-install value (e.g. the
/// `spreadsheet_id` the user picked) wins key by key, so a manifest may pin a field the install can't
/// override (a tab) while the install pins the resource the manifest left as a placeholder. A
/// non-object on either side replaces wholesale. Called before `build_agent`, so validation and the
/// gate see the real target, not the placeholder.
pub fn merge_bound(
    target: &mut Option<serde_json::Value>,
    binding: serde_json::Value,
) {
    match (target.as_mut().and_then(|t| t.as_object_mut()), binding) {
        (Some(base), serde_json::Value::Object(over)) => {
            for (k, v) in over {
                base.insert(k, v);
            }
        }
        (_, binding) => *target = Some(binding),
    }
}

fn preamble_for(manifest: &AgentManifest, conn: &ConnectorManifest) -> String {
    let summary = capability_summary(&manifest.governance, conn);
    if manifest.system_prompt.trim().is_empty() {
        summary
    } else {
        format!("{}\n\n{}", manifest.system_prompt, summary)
    }
}

/// A plain-language list of what the agent may do, derived from the governed tools resolved against
/// the connector plus the bound resource and the read-before-write floor. Mounted into the preamble
/// so the model knows its real capabilities and nothing more.
pub fn capability_summary(
    gov: &Governance,
    conn: &ConnectorManifest,
) -> String {
    let mut lines = vec![format!(
        "Connector: {} ({}). You may use only these tools:",
        conn.connector, conn.connector_type
    )];
    for tp in &gov.tools {
        if let Some(ct) = conn.tool(&tp.tool) {
            lines.push(format!(
                "- {}: {} on {} (mode {})",
                tp.tool,
                action_word(ct.action),
                ct.resource,
                mode_word(tp.mode),
            ));
        }
    }
    if let Some(bound) = &gov.bound_resource {
        lines.push(format!(
            "Act only on the bound resource {bound}. Never target another."
        ));
    }
    if gov.read_before_write {
        lines.push("Read the bound resource before any write.".to_string());
    }
    lines.join("\n")
}

fn action_word(a: crate::domain::governance::Action) -> &'static str {
    use crate::domain::governance::Action::*;
    match a {
        Search => "search",
        Read => "read",
        Append => "append",
        Update => "update",
        Upsert => "upsert",
        Create => "create",
        Clear => "clear",
        Delete => "delete",
    }
}

fn mode_word(m: crate::domain::governance::Mode) -> &'static str {
    use crate::domain::governance::Mode::*;
    match m {
        ReadOnly => "read_only",
        AppendOnly => "append_only",
        ReadWrite => "read_write",
        Upsert => "upsert",
    }
}
