use flowflow::domain::capability_contract::{
    validate_capability_owners, CapabilityRequirement, ToolOwner,
};
use flowflow::domain::governance::{Action, ConnectorManifest, Governance};
use serde_json::json;

fn fixtures() -> (
    Governance,
    Vec<CapabilityRequirement>,
    Vec<(String, ConnectorManifest)>,
) {
    let gov = serde_json::from_value(json!({"tools":[
        {"tool":"sheets_read","mode":"read_only"},
        {"tool":"docs_read","mode":"read_only"}]}))
    .unwrap();
    let reqs = ["sheets", "docs"]
        .into_iter()
        .map(|key| CapabilityRequirement {
            key: key.into(),
            connector_type: key.into(),
            capabilities: vec![Action::Read],
        })
        .collect();
    let resolved = ["sheets", "docs"].into_iter().map(|key| (key.into(), serde_json::from_value(json!({
        "connector":key,"type":key,"server":"https://example.invalid","mcp_prefix":format!("{key}_"),
        "provides":["read"],"tools":[{"tool":format!("{key}_read"),"resource":key,
        "action":"read","risk":"read_only"}]})).unwrap())).collect();
    (gov, reqs, resolved)
}

#[test]
fn validates_every_owner_independently_of_resolution_order() {
    let (gov, reqs, mut resolved) = fixtures();
    resolved.reverse();
    let owners =
        validate_capability_owners(&gov, &reqs, &resolved, &[]).unwrap();
    assert_eq!(owners["docs_read"], ToolOwner::Connector("docs".into()));
    assert_eq!(owners["sheets_read"], ToolOwner::Connector("sheets".into()));
    resolved[0].1.provides.clear();
    assert!(validate_capability_owners(&gov, &reqs, &resolved, &[]).is_err());
}

#[test]
fn missing_wrong_type_extra_and_duplicate_resolutions_are_refused() {
    let (gov, reqs, resolved) = fixtures();
    assert!(
        validate_capability_owners(&gov, &reqs, &resolved[..1], &[]).is_err()
    );
    for dimension in 0..3 {
        let mut bad = resolved.clone();
        match dimension {
            0 => bad[1].1.connector_type = "wrong".into(),
            1 => bad[1].0 = "extra".into(),
            _ => bad.push(resolved[1].clone()),
        }
        assert!(validate_capability_owners(&gov, &reqs, &bad, &[]).is_err());
    }
}

#[test]
fn native_only_and_mixed_plans_validate_without_fake_connectors() {
    let native: Governance = serde_json::from_value(json!({"tools":[
        {"tool":"create_note","mode":"read_write","approval":"require_approval"}]})).unwrap();
    let owners =
        validate_capability_owners(&native, &[], &[], &["create_note".into()])
            .unwrap();
    assert_eq!(owners["create_note"], ToolOwner::Native);
    let (mut gov, reqs, resolved) = fixtures();
    gov.tools.extend(native.tools);
    assert_eq!(
        validate_capability_owners(
            &gov,
            &reqs,
            &resolved,
            &["create_note".into()]
        )
        .unwrap()
        .len(),
        3
    );
    gov.tools.last_mut().unwrap().approval =
        flowflow::domain::governance::Approval::Auto;
    assert!(validate_capability_owners(
        &gov,
        &reqs,
        &resolved,
        &["create_note".into()]
    )
    .is_err());
}

#[test]
fn ambiguous_prefixes_tool_owners_native_collisions_and_unknown_tools_fail() {
    let (gov, reqs, resolved) = fixtures();
    for dimension in 0..4 {
        let mut bad = resolved.clone();
        match dimension {
            0 => bad[1].1.mcp_prefix = bad[0].1.mcp_prefix.clone(),
            1 => bad[1].1.tools = bad[0].1.tools.clone(),
            2 => bad[1].1.tools[0].tool = "create_note".into(),
            _ => bad[1].1.mcp_prefix = "wrong_".into(),
        }
        assert!(validate_capability_owners(&gov, &reqs, &bad, &[]).is_err());
    }
    assert!(validate_capability_owners(
        &gov,
        &reqs,
        &resolved,
        &["unknown_native".into()]
    )
    .is_err());
    let mut unknown = gov;
    unknown.tools[0].tool = "unknown_remote".into();
    assert!(
        validate_capability_owners(&unknown, &reqs, &resolved, &[]).is_err()
    );
}

#[test]
fn no_empty_execution_shared_bound_or_ungranted_native_declaration() {
    let (mut gov, reqs, resolved) = fixtures();
    assert!(validate_capability_owners(
        &gov,
        &reqs,
        &resolved,
        &["search_notes".into()]
    )
    .is_err());
    gov.bound_resource = Some(json!({"id":"global"}));
    assert!(validate_capability_owners(&gov, &reqs, &resolved, &[]).is_err());
    gov.bound_resource = None;
    gov.tools.clear();
    assert!(validate_capability_owners(&gov, &[], &[], &[]).is_err());
}
