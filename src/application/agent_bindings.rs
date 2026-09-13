//! Pure resource-binding adapter for the reader-first block-B rollout.
//! No wire deserialization, persistence migration or legacy runtime activation.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::domain::governance::{
    validate_governance, ConnectorManifest, Governance,
};

/// Supplied by the authenticated session and verified installed package, not
/// copied from an untrusted binding request. Device ids are globally unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingIdentity {
    pub device_id: String,
    pub agent_id: String,
    pub package_digest: String,
}

#[derive(Debug, Clone)]
pub struct ResourceBinding {
    pub identity: BindingIdentity,
    pub requirement_key: String,
    pub connector_slug: String,
    pub resource: Value,
}

/// Resolution happens before adaptation. Requirement keys are not provider slugs.
#[derive(Debug, Clone)]
pub struct ResolvedResourceRequirement {
    pub key: String,
    pub connector_slug: String,
    pub manifest: ConnectorManifest,
    pub resource_required: bool,
}

/// Adapt external-tool policies into the existing ContractHook input shape.
/// Callers must install all returned entries in ONE hook to share run budgets.
/// Native descriptors are validated separately; no unknown tool is ignored here.
pub fn scoped_resource_contracts(
    identity: &BindingIdentity,
    governance: &Governance,
    requirements: &[ResolvedResourceRequirement],
    bindings: &[ResourceBinding],
) -> Result<Vec<(String, Governance, ConnectorManifest)>, String> {
    if identity.device_id.trim().is_empty()
        || identity.agent_id.trim().is_empty()
        || identity.package_digest.trim().is_empty()
    {
        return Err("binding identity must be complete".into());
    }
    // Reusing a legacy shared bound would leak one owner's destination into
    // another owner's contract. New callers must provide only keyed selections.
    if governance.bound_resource.is_some() {
        return Err(
            "scoped bindings cannot use a legacy shared bound_resource".into(),
        );
    }
    let mut by_key = BTreeMap::new();
    let mut prefixes = BTreeSet::new();
    let mut slugs = BTreeSet::new();
    let mut tool_owners = BTreeMap::new();
    for requirement in requirements {
        if requirement.key.trim().is_empty()
            || requirement.connector_slug.trim().is_empty()
            || requirement.manifest.mcp_prefix.trim().is_empty()
        {
            return Err("resolved requirement identity must be complete".into());
        }
        if by_key
            .insert(requirement.key.as_str(), requirement)
            .is_some()
        {
            return Err(format!("duplicate requirement `{}`", requirement.key));
        }
        // The current hook routes by prefix and tool name. Two independent
        // bindings to the same peer cannot yet be routed without ambiguity.
        if !prefixes.insert(requirement.manifest.mcp_prefix.as_str())
            || !slugs.insert(requirement.connector_slug.as_str())
        {
            return Err("resolved connector routes must be unique".into());
        }
        for tool in &requirement.manifest.tools {
            if crate::application::tools::NOTES_TOOL_NAMES
                .contains(&tool.tool.as_str())
            {
                return Err(format!(
                    "external connector claims native tool `{}`",
                    tool.tool
                ));
            }
            if tool_owners
                .insert(tool.tool.as_str(), requirement.key.as_str())
                .is_some()
            {
                return Err(format!("ambiguous tool owner `{}`", tool.tool));
            }
        }
    }
    // ContractHook uses longest-prefix routing, not the ownership map above.
    // Check that the runtime will select the same owner for every advertised tool.
    for (tool, owner) in &tool_owners {
        let routed = requirements
            .iter()
            .filter(|requirement| {
                tool.starts_with(&requirement.manifest.mcp_prefix)
            })
            .max_by_key(|requirement| requirement.manifest.mcp_prefix.len());
        if routed.is_none_or(|requirement| requirement.key != *owner) {
            return Err(format!(
                "tool `{tool}` does not route to its declared owner"
            ));
        }
    }
    let mut granted = BTreeSet::new();
    for policy in &governance.tools {
        if !granted.insert(policy.tool.as_str()) {
            return Err(format!("duplicate tool policy `{}`", policy.tool));
        }
        if !tool_owners.contains_key(policy.tool.as_str()) {
            return Err(format!(
                "tool `{}` has no resolved owner",
                policy.tool
            ));
        }
    }
    let mut selections = BTreeMap::new();
    for binding in bindings {
        if &binding.identity != identity {
            return Err(
                "binding belongs to another device or installed package".into(),
            );
        }
        let requirement = by_key
            .get(binding.requirement_key.as_str())
            .ok_or_else(|| {
                format!(
                    "unknown binding requirement `{}`",
                    binding.requirement_key
                )
            })?;
        if binding.connector_slug != requirement.connector_slug {
            return Err(format!(
                "binding owner changed for `{}`",
                binding.requirement_key
            ));
        }
        if selections
            .insert(binding.requirement_key.as_str(), &binding.resource)
            .is_some()
        {
            return Err(format!(
                "duplicate binding `{}`",
                binding.requirement_key
            ));
        }
    }
    requirements
        .iter()
        .map(|requirement| {
            let selected = selections
                .get(requirement.key.as_str())
                .copied()
                .ok_or_else(|| {
                    format!("missing owner selection for `{}`", requirement.key)
                })?;
            let resource = if requirement.resource_required {
                if selected.is_null() {
                    return Err(format!(
                        "missing resource for `{}`",
                        requirement.key
                    ));
                }
                Some(selected.clone())
            } else {
                if !selected.is_null() {
                    return Err(format!(
                        "resource-free owner `{}` requires null",
                        requirement.key
                    ));
                }
                None
            };
            let scoped = Governance {
                tools: governance
                    .tools
                    .iter()
                    .filter(|policy| {
                        requirement.manifest.tool(&policy.tool).is_some()
                    })
                    .cloned()
                    .collect(),
                bound_resource: resource,
                ..governance.clone()
            };
            validate_governance(&scoped, &requirement.manifest).map_err(
                |errors| {
                    format!(
                        "requirement `{}`: {}",
                        requirement.key,
                        errors
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                },
            )?;
            Ok((
                requirement.manifest.mcp_prefix.clone(),
                scoped,
                requirement.manifest.clone(),
            ))
        })
        .collect()
}
