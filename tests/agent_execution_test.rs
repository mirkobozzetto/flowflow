use flowflow::application::agent_bindings::{
    BindingIdentity, ResolvedResourceRequirement, ResourceBinding,
};
use flowflow::application::agent_execution::{
    assemble_scoped_run, ScopedExecutionInput,
};
use flowflow::application::agent_native::NativeCapability;
use flowflow::domain::capability_contract::CapabilityRequirement;
use flowflow::domain::governance::{Action, Governance};
use serde_json::json;
use tokio::sync::mpsc;

#[test]
fn native_only_is_assembled_without_external_connectors() {
    let id = BindingIdentity {
        device_id: "d".into(),
        agent_id: "a".into(),
        package_digest: "digest".into(),
    };
    let gov: Governance = serde_json::from_value(
        json!({"tools":[{"tool":"search_notes","mode":"read_only"}]}),
    )
    .unwrap();
    let (tx, _rx) = mpsc::unbounded_channel();
    let run = assemble_scoped_run(
        ScopedExecutionInput {
            identity: &id,
            governance: &gov,
            requirements: &[],
            resolved: &[],
            bindings: &[],
            native_tools: &["search_notes".into()],
            available_native: &[NativeCapability::SearchNotes],
        },
        tx,
    )
    .unwrap();
    assert!(!run.aborted());
}

#[test]
fn mixed_assembly_requires_all_capabilities_bindings_and_write_decision_channel(
) {
    let id = BindingIdentity {
        device_id: "d".into(),
        agent_id: "a".into(),
        package_digest: "digest".into(),
    };
    let gov: Governance = serde_json::from_value(json!({"tools":[
        {"tool":"remote_read","mode":"read_only"},
        {"tool":"create_note","mode":"read_write","approval":"require_approval"}]})).unwrap();
    let requirements = [CapabilityRequirement {
        key: "source".into(),
        connector_type: "documents".into(),
        capabilities: vec![Action::Read],
    }];
    let resolved = [ResolvedResourceRequirement { key:"source".into(), connector_slug:"provider".into(), resource_required:true,
        manifest: serde_json::from_value(json!({"connector":"provider","type":"documents","server":"https://example.invalid",
            "mcp_prefix":"remote_","provides":["read"],"tools":[{"tool":"remote_read","resource":"document","action":"read","risk":"read_only"}]})).unwrap() }];
    let bindings = [ResourceBinding {
        identity: id.clone(),
        requirement_key: "source".into(),
        connector_slug: "provider".into(),
        resource: json!({"document_id":"doc-1"}),
    }];
    let native = ["create_note".into()];
    let available = [NativeCapability::CreateNote];
    let input = |bound| ScopedExecutionInput {
        identity: &id,
        governance: &gov,
        requirements: &requirements,
        resolved: &resolved,
        bindings: bound,
        native_tools: &native,
        available_native: &available,
    };
    let (tx, rx) = mpsc::unbounded_channel();
    assert!(assemble_scoped_run(input(&bindings), tx.clone()).is_ok());
    assert!(assemble_scoped_run(input(&[]), tx.clone()).is_err());
    let mut stale = bindings.clone();
    stale[0].identity.device_id = "another-device".into();
    assert!(assemble_scoped_run(input(&stale), tx.clone()).is_err());
    drop(rx);
    assert!(assemble_scoped_run(input(&bindings), tx).is_err());
}
