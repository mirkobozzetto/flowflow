//! Assemble a scoped run only after every external/native owner and binding passes.
//! Package verification and schema dispatch precede this internal seam.

use tokio::sync::mpsc::UnboundedSender;

use crate::application::agent_bindings::{
    scoped_resource_contracts, BindingIdentity, ResolvedResourceRequirement,
    ResourceBinding,
};
use crate::application::agent_native::{
    validate_native_capabilities, NativeCapability,
};
use crate::application::agent_run::ScopedAgentRun;
use crate::application::tools::ToolEvent;
use crate::domain::capability_contract::{
    validate_capability_owners, CapabilityRequirement, ToolOwner,
};
use crate::domain::governance::Governance;

/// Inputs originate from a verified package, authenticated device and actual
/// mounted connector/native capabilities. No network or storage writes occur.
/// Never substitute this constructor for signature verification or entitlement.
pub struct ScopedExecutionInput<'a> {
    pub identity: &'a BindingIdentity,
    pub governance: &'a Governance,
    pub requirements: &'a [CapabilityRequirement],
    pub resolved: &'a [ResolvedResourceRequirement],
    pub bindings: &'a [ResourceBinding],
    pub native_tools: &'a [String],
    pub available_native: &'a [NativeCapability],
}

pub fn assemble_scoped_run(
    input: ScopedExecutionInput<'_>,
    events: UnboundedSender<ToolEvent>,
) -> Result<ScopedAgentRun, String> {
    let resolved: Vec<_> = input
        .resolved
        .iter()
        .map(|requirement| {
            (requirement.key.clone(), requirement.manifest.clone())
        })
        .collect();
    let owners = validate_capability_owners(
        input.governance,
        input.requirements,
        &resolved,
        input.native_tools,
    )?;
    let native_policies: Vec<_> = input
        .governance
        .tools
        .iter()
        .filter(|policy| owners.get(&policy.tool) == Some(&ToolOwner::Native))
        .cloned()
        .collect();
    let native = validate_native_capabilities(
        &native_policies,
        input.available_native,
        !events.is_closed(),
    )?;
    let external_governance = Governance {
        tools: input
            .governance
            .tools
            .iter()
            .filter(|policy| {
                matches!(
                    owners.get(&policy.tool),
                    Some(ToolOwner::Connector(_))
                )
            })
            .cloned()
            .collect(),
        ..input.governance.clone()
    };
    let entries = scoped_resource_contracts(
        input.identity,
        &external_governance,
        input.resolved,
        input.bindings,
    )?;
    Ok(ScopedAgentRun::new(
        events,
        entries,
        native,
        input.governance.limits.clone().unwrap_or_default(),
    ))
}
