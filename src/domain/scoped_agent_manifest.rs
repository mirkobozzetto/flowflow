//! Strict schema-2 reader. The schema-1 reader remains unchanged.

use crate::domain::agent_manifest::{canonical_json, verify_envelope};
use crate::domain::capability_contract::{
    native_action, validate_capability_owners,
};
use crate::domain::governance::{Action, Approval, Governance};
use crate::domain::orchestration::Orchestration;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedAgentManifest {
    pub schema_version: String,
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub alias: String,
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub metadata: Value,
    pub execution: ScopedExecution,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedExecution {
    pub required_connectors: Vec<ScopedRequirement>,
    pub native_tools: Vec<String>,
    pub governance: Governance,
    #[serde(default)]
    pub orchestration: Orchestration,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedRequirement {
    pub key: String,
    #[serde(rename = "type")]
    pub connector_type: String,
    pub capabilities: Vec<Action>,
    pub resource_required: bool,
}

/// Private fields prevent pinning a caller-created value that skipped integrity.
#[derive(Debug)]
pub struct VerifiedScopedAgent {
    manifest: ScopedAgentManifest,
    digest: String,
    canonical: String,
}
impl VerifiedScopedAgent {
    pub fn manifest(&self) -> &ScopedAgentManifest {
        &self.manifest
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn canonical(&self) -> &str {
        &self.canonical
    }
}

fn known_fields(
    value: &Value,
    allowed: &[&str],
    path: &str,
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported execution field {path}.{key}"));
        }
    }
    Ok(())
}

pub fn parse_scoped_manifest(raw: &str) -> Result<ScopedAgentManifest, String> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| format!("manifest JSON: {error}"))?;
    let execution = &value["execution"];
    let governance = &execution["governance"];
    // Shared legacy structs tolerate extra fields; a new execution reader must
    // not silently discard policy semantics before those structs deserialize.
    known_fields(
        governance,
        &["tools", "read_before_write", "deny_destructive", "limits"],
        "execution.governance",
    )?;
    if let Some(policies) = governance["tools"].as_array() {
        for policy in policies {
            known_fields(
                policy,
                &["tool", "mode", "approval", "max_calls_per_run"],
                "execution.governance.tools[]",
            )?;
        }
    }
    if let Some(limits) = governance.get("limits") {
        known_fields(
            limits,
            &["max_steps", "max_tool_calls", "max_run_seconds"],
            "execution.governance.limits",
        )?;
    }
    if let Some(orchestration) = execution.get("orchestration") {
        known_fields(
            orchestration,
            &["chains", "triggers"],
            "execution.orchestration",
        )?;
        if let Some(triggers) = orchestration["triggers"].as_array() {
            for trigger in triggers {
                known_fields(
                    trigger,
                    &["kind", "scope", "words", "then"],
                    "execution.orchestration.triggers[]",
                )?;
            }
        }
    }
    let manifest: ScopedAgentManifest = serde_json::from_value(value)
        .map_err(|error| format!("schema-2 manifest: {error}"))?;
    if manifest.schema_version != "2" {
        return Err("scoped reader requires schema_version 2".into());
    }
    if [
        &manifest.id,
        &manifest.version,
        &manifest.name,
        &manifest.model,
    ]
    .iter()
    .any(|field| field.trim().is_empty())
    {
        return Err(
            "agent id, version, name and model must be non-empty".into()
        );
    }
    if manifest.temperature.is_some_and(|value| !value.is_finite()) {
        return Err("temperature must be finite".into());
    }
    let mut requirements = BTreeSet::new();
    for requirement in &manifest.execution.required_connectors {
        if requirement.key.trim().is_empty()
            || requirement.connector_type.trim().is_empty()
            || !requirements.insert(&requirement.key)
        {
            return Err("requirement keys must be non-empty and unique; types must be non-empty".into());
        }
    }
    let mut policies = BTreeSet::new();
    for policy in &manifest.execution.governance.tools {
        if policy.tool.trim().is_empty()
            || !policies.insert(policy.tool.as_str())
        {
            return Err("tool policies must be non-empty and unique".into());
        }
    }
    if policies.is_empty() {
        return Err("execution declares no tools".into());
    }
    let mut natives = BTreeSet::new();
    for tool in &manifest.execution.native_tools {
        let action = native_action(tool)
            .ok_or_else(|| format!("unknown native tool {tool}"))?;
        if !natives.insert(tool.as_str()) {
            return Err("duplicate native declaration".into());
        }
        let policy = manifest
            .execution
            .governance
            .tools
            .iter()
            .find(|policy| &policy.tool == tool)
            .ok_or_else(|| format!("native tool {tool} has no policy"))?;
        if !policy.mode.allows(action)
            || (action.is_write()
                && policy.approval != Approval::RequireApproval)
        {
            return Err(format!("native policy {tool} does not satisfy mode/approval requirements"));
        }
    }
    for policy in &manifest.execution.governance.tools {
        if native_action(&policy.tool).is_some()
            && !natives.contains(policy.tool.as_str())
        {
            return Err(format!("native tool {} is not declared", policy.tool));
        }
    }
    if requirements.is_empty() {
        validate_capability_owners(
            &manifest.execution.governance,
            &[],
            &[],
            &manifest.execution.native_tools,
        )?;
    }
    for chain in manifest.execution.orchestration.chains.values() {
        chain.validate().map_err(|error| error.to_string())?;
        for state in chain.states.values() {
            if state
                .allowed_tools
                .iter()
                .any(|tool| !policies.contains(tool.as_str()))
            {
                return Err("chain exposes an ungoverned tool".into());
            }
        }
    }
    for trigger in &manifest.execution.orchestration.triggers {
        if trigger.kind != "keyword"
            || trigger.scope != "in_run"
            || trigger.words.is_empty()
            || trigger.words.iter().any(|word| word.trim().is_empty())
            || !manifest
                .execution
                .orchestration
                .chains
                .contains_key(&trigger.then)
        {
            return Err("unsupported or invalid in-run keyword trigger".into());
        }
    }
    Ok(manifest)
}

pub fn verify_scoped_package(
    package: &str,
    public_key: &str,
) -> Result<VerifiedScopedAgent, String> {
    let envelope: Value = serde_json::from_str(package)
        .map_err(|error| format!("package JSON: {error}"))?;
    let (value, digest) = verify_envelope(&envelope, public_key)
        .map_err(|error| error.to_string())?;
    let canonical = canonical_json(&value);
    let manifest = parse_scoped_manifest(&canonical)?;
    Ok(VerifiedScopedAgent {
        manifest,
        digest,
        canonical,
    })
}
