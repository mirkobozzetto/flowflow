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

// The local web search tool. An agent opts into web access by governing this name; it runs
// on-device against the user's own Exa key, so no connector serves it.
pub const WEB_SEARCH_TOOL: &str = "search_web";

// One resolved connector of a built agent: the slug the device dials and the manifest its calls
// are governed against.
#[derive(Debug, Clone)]
pub struct BoundConnector {
    pub slug: String,
    pub manifest: ConnectorManifest,
}

// The resolved, runnable contract handed to the chain runtime / module path. `model` is surfaced for
// when per-agent model selection lands; the run still uses the provider-configured LlmClient today.
// An agent may require several connectors (search the web, then write the sheet); they are resolved
// in manifest order and the first one stays the default for single-connector paths.
#[derive(Debug)]
pub struct BuiltAgent {
    pub agent_id: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub preamble: String,
    // The manifest's own instruction, alone. `preamble` bundles it with the capability summary,
    // which lists every tool the agent governs: injected into a chain state that allows only a
    // subset, that list contradicts the state's restriction, and at the tool-less answer step it
    // is pure hallucination fuel. The chain runtime needs the mission and nothing else.
    pub mission: String,
    pub governance: Governance,
    pub connectors: Vec<BoundConnector>,
    pub chains: BTreeMap<String, Chain>,
}

impl BuiltAgent {
    /// The first resolved connector. Every agent has at least one: the build refuses otherwise.
    pub fn primary(&self) -> &BoundConnector {
        self.connectors
            .first()
            .expect("built agent has a connector")
    }

    pub fn slug(&self) -> &str {
        &self.primary().slug
    }

    pub fn connector(&self) -> &ConnectorManifest {
        &self.primary().manifest
    }

    /// Whether the manifest governs the local web search tool. Declaring it is what opts an
    /// agent into web access; it is served by no connector, so the gate never routes it.
    pub fn governs_web_search(&self) -> bool {
        self.governance
            .tools
            .iter()
            .any(|t| t.tool == WEB_SEARCH_TOOL)
    }

    /// Every connector slug to dial for a run, in manifest order.
    pub fn slugs(&self) -> Vec<String> {
        self.connectors.iter().map(|c| c.slug.clone()).collect()
    }

    /// One (mcp_prefix, governance, manifest) entry per connector, for the gating hook. The
    /// governance block is agent-level, so each connector is gated against the same rules over
    /// its own tool set.
    pub fn contract_entries(
        &self,
    ) -> Vec<(String, Governance, ConnectorManifest)> {
        self.connectors
            .iter()
            .map(|c| {
                (
                    c.manifest.mcp_prefix.clone(),
                    self.governance.clone(),
                    c.manifest.clone(),
                )
            })
            .collect()
    }
}

/// The connector TYPES this manifest requires, in declaration order. The caller resolves each to a
/// pinned (slug, manifest) via `connector_pins` - data, not a compiled match (RFC 0016) - and hands
/// the resolutions in, so the build stays pure.
pub fn required_connector_types(
    manifest: &AgentManifest,
) -> Result<Vec<&str>, String> {
    let types: Vec<&str> = manifest
        .required_connectors
        .iter()
        .map(|r| r.connector_type.as_str())
        .collect();
    if types.is_empty() {
        return Err("manifest declares no required_connectors".to_string());
    }
    Ok(types)
}

/// Validate a manifest against its RESOLVED connectors into a runnable agent. Fails closed:
/// governance that does not validate against the connectors, an unsound chain, or two connectors
/// serving the same tool name (the call would have no unambiguous owner) stop the build.
pub fn build_agent_multi(
    manifest: &AgentManifest,
    resolved: &[(String, String)],
) -> Result<BuiltAgent, String> {
    if resolved.is_empty() {
        return Err("no connector resolved for this agent".to_string());
    }
    let mut connectors = Vec::with_capacity(resolved.len());
    for (slug, conn_json) in resolved {
        let manifest = parse_connector_manifest(conn_json)
            .map_err(|e| format!("connector manifest `{slug}`: {e}"))?;
        connectors.push(BoundConnector {
            slug: slug.clone(),
            manifest,
        });
    }

    if let Some(dupe) = duplicate_tool(&connectors) {
        return Err(format!(
            "two connectors serve the tool `{dupe}`: a call would have no unambiguous owner"
        ));
    }

    // A tool the agent governs must exist on ONE of the connectors; validating against each
    // connector separately would reject every tool served by the others.
    validate_governance_across(&manifest.governance, &connectors)?;

    for (name, chain) in &manifest.orchestration.chains {
        chain
            .validate()
            .map_err(|e| format!("chain `{name}`: {e}"))?;
    }

    Ok(BuiltAgent {
        agent_id: manifest.id.clone(),
        model: manifest.model.clone(),
        temperature: manifest.temperature,
        preamble: preamble_for_all(manifest, &connectors),
        mission: manifest.system_prompt.trim().to_string(),
        governance: manifest.governance.clone(),
        connectors,
        chains: manifest.orchestration.chains.clone(),
    })
}

/// Single-connector build, the common case.
pub fn build_agent(
    manifest: &AgentManifest,
    slug: &str,
    conn_json: &str,
) -> Result<BuiltAgent, String> {
    build_agent_multi(manifest, &[(slug.to_string(), conn_json.to_string())])
}

// A tool name served by more than one connector.
fn duplicate_tool(connectors: &[BoundConnector]) -> Option<String> {
    let mut seen = std::collections::BTreeSet::new();
    for c in connectors {
        for tool in &c.manifest.tools {
            if !seen.insert(tool.tool.clone()) {
                return Some(tool.tool.clone());
            }
        }
    }
    None
}

// Governance holds for the agent as a whole: each governed tool is validated against the connector
// that serves it. A tool no connector serves fails the build, as it did with one connector.
fn validate_governance_across(
    gov: &Governance,
    connectors: &[BoundConnector],
) -> Result<(), String> {
    let mut errors = Vec::new();
    for c in connectors {
        let mine: Vec<_> = gov
            .tools
            .iter()
            .filter(|tp| c.manifest.tool(&tp.tool).is_some())
            .cloned()
            .collect();
        let scoped = Governance {
            tools: mine,
            ..gov.clone()
        };
        if let Err(errs) = validate_governance(&scoped, &c.manifest) {
            errors.extend(errs.iter().map(|e| e.to_string()));
        }
    }
    let unknown: Vec<_> = gov
        .tools
        .iter()
        .filter(|tp| tp.tool != WEB_SEARCH_TOOL)
        .filter(|tp| {
            !connectors
                .iter()
                .any(|c| c.manifest.tool(&tp.tool).is_some())
        })
        .map(|tp| format!("tool `{}` is served by no connector", tp.tool))
        .collect();
    errors.extend(unknown);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("governance invalid: {}", errors.join("; ")))
    }
}

/// Merge an arm-time binding over the manifest's `bound_resource`. The per-install value (e.g. the
/// `spreadsheet_id` the user picked) wins key by key, so the install pins the resource the manifest
/// left as a placeholder. A v2 array binding composes the same way per ENTRY: each entry inherits
/// the manifest object's fields, its own fields winning. A non-object on either side (or an array
/// binding over a non-object target) replaces wholesale. Called before `build_agent`, so validation
/// and the gate see the real target, not the placeholder.
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
        (Some(base), serde_json::Value::Array(entries)) => {
            let merged: Vec<serde_json::Value> = entries
                .into_iter()
                .map(|e| match e {
                    serde_json::Value::Object(over) => {
                        let mut entry = base.clone();
                        for (k, v) in over {
                            entry.insert(k, v);
                        }
                        serde_json::Value::Object(entry)
                    }
                    other => other,
                })
                .collect();
            *target = Some(serde_json::Value::Array(merged));
        }
        (_, binding) => *target = Some(binding),
    }
}

fn preamble_for_all(
    manifest: &AgentManifest,
    connectors: &[BoundConnector],
) -> String {
    let summary = connectors
        .iter()
        .map(|c| capability_summary(&manifest.governance, &c.manifest))
        .collect::<Vec<_>>()
        .join("\n\n");
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
