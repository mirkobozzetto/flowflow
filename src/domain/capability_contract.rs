//! Pure, pre-network admission mirrored by server and device.
//! New execution contracts only; legacy schema-1 validation is unchanged.

use std::collections::{BTreeMap, BTreeSet};

use crate::domain::governance::{
    validate_governance, Action, Approval, ConnectorManifest, Governance,
};

#[derive(Debug, Clone)]
pub struct CapabilityRequirement {
    pub key: String,
    pub connector_type: String,
    pub capabilities: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolOwner {
    Connector(String),
    Native,
}

/// Native descriptors have no server, connector slug or OAuth dependency.
/// Mount availability and actual approval delivery are runtime checks.
pub fn native_action(name: &str) -> Option<Action> {
    match name {
        "search_notes" | "search_web" => Some(Action::Search),
        "summarize_folder" => Some(Action::Read),
        "create_note" | "schedule_reminder" => Some(Action::Create),
        _ => None,
    }
}

/// `resolved` is keyed by logical requirement, never declaration order. Validate
/// every owner and return the governed-tool map only after the full set passes.
/// Resource selection is checked later by the isolated binding adapter; no
/// placeholder bound is invented to make an unbound draft look executable.
pub fn validate_capability_owners(
    governance: &Governance,
    requirements: &[CapabilityRequirement],
    resolved: &[(String, ConnectorManifest)],
    native_tools: &[String],
) -> Result<BTreeMap<String, ToolOwner>, String> {
    if governance.tools.is_empty() {
        return Err("execution declares no tools".into());
    }
    if governance.bound_resource.is_some() {
        return Err(
            "new execution cannot declare a shared bound_resource".into()
        );
    }
    if governance.column_roles.is_some()
        || governance.on_multiple_match.is_some()
    {
        return Err(
            "shared spreadsheet column policies need an explicit owner".into(),
        );
    }
    let mut declarations = BTreeMap::new();
    for requirement in requirements {
        if requirement.key.trim().is_empty()
            || requirement.connector_type.trim().is_empty()
        {
            return Err("requirement key and type must be non-empty".into());
        }
        if declarations
            .insert(requirement.key.as_str(), requirement)
            .is_some()
        {
            return Err(format!("duplicate requirement `{}`", requirement.key));
        }
    }
    let mut owners = BTreeMap::new();
    let mut matched = BTreeSet::new();
    let mut prefixes = BTreeSet::new();
    for (key, connector) in resolved {
        let requirement = declarations.get(key.as_str()).ok_or_else(|| {
            format!("undeclared resolved requirement `{key}`")
        })?;
        if !matched.insert(key.as_str()) {
            return Err(format!("duplicate resolution `{key}`"));
        }
        if connector.connector_type != requirement.connector_type {
            return Err(format!(
                "requirement `{key}` resolved to the wrong type"
            ));
        }
        for capability in &requirement.capabilities {
            if !connector.provides.contains(capability) {
                return Err(format!(
                    "requirement `{key}` does not provide {capability:?}"
                ));
            }
        }
        if connector.mcp_prefix.trim().is_empty()
            || !prefixes.insert(connector.mcp_prefix.as_str())
        {
            return Err(
                "connector prefixes must be non-empty and unique".into()
            );
        }
        for tool in &connector.tools {
            if native_action(&tool.tool).is_some() {
                return Err(format!(
                    "connector claims native tool `{}`",
                    tool.tool
                ));
            }
            if !connector.provides.contains(&tool.action) {
                return Err(format!(
                    "connector tool `{}` has an undeclared action",
                    tool.tool
                ));
            }
            if owners
                .insert(tool.tool.clone(), ToolOwner::Connector(key.clone()))
                .is_some()
            {
                return Err(format!("ambiguous tool owner `{}`", tool.tool));
            }
        }
        let scoped = Governance {
            tools: governance
                .tools
                .iter()
                .filter(|policy| connector.tool(&policy.tool).is_some())
                .cloned()
                .collect(),
            // No device resource exists at draft admission. The binding adapter
            // rechecks the original read-before-write policy before execution.
            read_before_write: false,
            ..governance.clone()
        };
        if let Err(errors) = validate_governance(&scoped, connector) {
            return Err(errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "));
        }
    }
    if matched.len() != declarations.len() {
        return Err("not every required connector was resolved".into());
    }
    for (tool, owner) in &owners {
        let routed = resolved
            .iter()
            .filter(|(_, connector)| tool.starts_with(&connector.mcp_prefix))
            .max_by_key(|(_, connector)| connector.mcp_prefix.len());
        if routed
            .is_none_or(|(key, _)| owner != &ToolOwner::Connector(key.clone()))
        {
            return Err(format!("tool `{tool}` routes to the wrong owner"));
        }
    }
    let mut natives = BTreeSet::new();
    for tool in native_tools {
        if native_action(tool).is_none() {
            return Err(format!("unsupported native tool `{tool}`"));
        }
        if !natives.insert(tool.as_str()) {
            return Err(format!("duplicate native tool `{tool}`"));
        }
        owners.insert(tool.clone(), ToolOwner::Native);
    }
    let mut admitted = BTreeMap::new();
    for policy in &governance.tools {
        let owner = owners.get(&policy.tool).ok_or_else(|| {
            format!("tool `{}` has no declared owner", policy.tool)
        })?;
        if admitted
            .insert(policy.tool.clone(), owner.clone())
            .is_some()
        {
            return Err(format!("duplicate tool policy `{}`", policy.tool));
        }
        if *owner == ToolOwner::Native {
            let action = native_action(&policy.tool)
                .expect("native declaration checked");
            if !policy.mode.allows(action) || policy.key_columns.is_some() {
                return Err(format!("invalid native policy `{}`", policy.tool));
            }
            if action.is_write() && policy.approval != Approval::RequireApproval
            {
                return Err(format!(
                    "native write `{}` requires user approval",
                    policy.tool
                ));
            }
        }
    }
    if natives.iter().any(|name| !admitted.contains_key(*name)) {
        return Err("native declaration has no governed policy".into());
    }
    Ok(admitted)
}
