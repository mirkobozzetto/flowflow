use flowflow::application::agent_bindings::{
    scoped_resource_contracts, BindingIdentity, ResolvedResourceRequirement,
    ResourceBinding,
};
use flowflow::domain::governance::Governance;
use serde_json::json;

fn fixture() -> (
    BindingIdentity,
    Governance,
    Vec<ResolvedResourceRequirement>,
    Vec<ResourceBinding>,
) {
    let identity = BindingIdentity {
        device_id: "device-a".into(),
        agent_id: "agent-a".into(),
        package_digest: format!("sha256:{}", "a".repeat(64)),
    };
    let requirements: Vec<_> = [("sheet", "sheets", "sheet_read"), ("document", "docs", "doc_read")]
        .into_iter().map(|(key, slug, tool)| ResolvedResourceRequirement {
            key: key.into(), connector_slug: slug.into(), resource_required: true,
            manifest: serde_json::from_value(json!({"connector":slug,"type":slug,
                "server":"https://example.invalid/mcp","mcp_prefix":format!("{}_", tool.split('_').next().unwrap()),"provides":["read"],
                "tools":[{"tool":tool,"resource":key,"action":"read","risk":"read_only"}]})).unwrap(),
        }).collect();
    let governance = serde_json::from_value(json!({"tools":[
        {"tool":"sheet_read","mode":"read_only"}, {"tool":"doc_read","mode":"read_only"}],
        "limits":{"max_steps":4,"max_tool_calls":7}})).unwrap();
    let bindings = requirements
        .iter()
        .map(|r| ResourceBinding {
            identity: identity.clone(),
            requirement_key: r.key.clone(),
            connector_slug: r.connector_slug.clone(),
            resource: if r.key == "sheet" {
                json!({"spreadsheet_id":"sheet-1"})
            } else {
                json!({"document_id":"doc-1"})
            },
        })
        .collect();
    (identity, governance, requirements, bindings)
}

#[test]
fn adapters_isolate_owner_resources_and_keep_run_policy() {
    let (id, gov, reqs, bindings) = fixture();
    let before = serde_json::to_value(&gov).unwrap();
    let contracts =
        scoped_resource_contracts(&id, &gov, &reqs, &bindings).unwrap();
    assert_eq!(contracts.len(), 2);
    for (i, (_, scoped, _)) in contracts.iter().enumerate() {
        assert_eq!(scoped.bound_resource.as_ref(), Some(&bindings[i].resource));
        assert_eq!(scoped.tools.len(), 1);
        assert_eq!(scoped.tools[0].tool, reqs[i].manifest.tools[0].tool);
        assert_eq!(scoped.limits.as_ref().unwrap().max_tool_calls, Some(7));
        assert!(scoped.read_before_write && scoped.deny_destructive);
    }
    assert_eq!(serde_json::to_value(&gov).unwrap(), before);
}

#[test]
fn refuses_cross_device_agent_digest_and_owner_reuse() {
    let (id, gov, reqs, bindings) = fixture();
    for dimension in 0..4 {
        let mut wrong = bindings.clone();
        match dimension {
            0 => wrong[0].identity.device_id = "device-b".into(),
            1 => wrong[0].identity.agent_id = "agent-b".into(),
            2 => wrong[0].identity.package_digest = "sha256:changed".into(),
            _ => wrong[0].connector_slug = "another-provider".into(),
        }
        assert!(scoped_resource_contracts(&id, &gov, &reqs, &wrong).is_err());
    }
}

#[test]
fn refuses_shared_bound_missing_extra_duplicate_and_malformed_selections() {
    let (id, mut gov, reqs, bindings) = fixture();
    gov.bound_resource = Some(json!({"spreadsheet_id":"legacy"}));
    assert!(scoped_resource_contracts(&id, &gov, &reqs, &bindings).is_err());
    gov.bound_resource = None;
    assert!(
        scoped_resource_contracts(&id, &gov, &reqs, &bindings[..1]).is_err()
    );
    let mut extra = bindings.clone();
    extra[1].requirement_key = "unknown".into();
    assert!(scoped_resource_contracts(&id, &gov, &reqs, &extra).is_err());
    let mut duplicate = bindings.clone();
    duplicate.push(bindings[0].clone());
    assert!(scoped_resource_contracts(&id, &gov, &reqs, &duplicate).is_err());
    for value in [json!(null), json!({}), json!([]), json!({"id":9})] {
        let mut malformed = bindings.clone();
        malformed[0].resource = value;
        assert!(
            scoped_resource_contracts(&id, &gov, &reqs, &malformed).is_err()
        );
    }
}

#[test]
fn refuses_ambiguous_routes_requirements_tools_and_unknown_policies() {
    let (id, gov, reqs, bindings) = fixture();
    for dimension in 0..4 {
        let mut ambiguous = reqs.clone();
        match dimension {
            0 => ambiguous[1].key = ambiguous[0].key.clone(),
            1 => {
                ambiguous[1].connector_slug =
                    ambiguous[0].connector_slug.clone()
            }
            2 => {
                ambiguous[1].manifest.mcp_prefix =
                    ambiguous[0].manifest.mcp_prefix.clone()
            }
            _ => {
                ambiguous[1].manifest.tools =
                    ambiguous[0].manifest.tools.clone()
            }
        }
        assert!(scoped_resource_contracts(&id, &gov, &ambiguous, &bindings)
            .is_err());
    }
    let mut unknown = gov.clone();
    unknown.tools[0].tool = "undeclared_native_write".into();
    assert!(scoped_resource_contracts(&id, &unknown, &reqs, &bindings).is_err());
    let mut duplicate = gov.clone();
    duplicate.tools.push(gov.tools[0].clone());
    assert!(
        scoped_resource_contracts(&id, &duplicate, &reqs, &bindings).is_err()
    );
}

#[test]
fn unbound_optional_owner_never_inherits_another_owners_resource() {
    let (id, gov, mut reqs, bindings) = fixture();
    reqs[1].resource_required = false;
    let contracts =
        scoped_resource_contracts(&id, &gov, &reqs, &bindings[..1]).unwrap();
    assert_eq!(
        contracts[0].1.bound_resource.as_ref(),
        Some(&bindings[0].resource)
    );
    assert!(contracts[1].1.bound_resource.is_none());
}

#[test]
fn refuses_a_prefix_that_would_route_to_no_owner_or_the_wrong_owner() {
    let (id, gov, mut reqs, bindings) = fixture();
    reqs[0].manifest.mcp_prefix = "unrelated_".into();
    assert!(scoped_resource_contracts(&id, &gov, &reqs, &bindings).is_err());
    reqs[0].manifest.mcp_prefix = "sheet_".into();
    reqs[1].manifest.mcp_prefix = "sheet_read".into();
    assert!(scoped_resource_contracts(&id, &gov, &reqs, &bindings).is_err());
}

#[tokio::test]
async fn real_hook_enforces_owner_bounds_and_a_shared_call_budget() {
    use flowflow::application::tools::ContractHook;
    use rig::agent::{PromptHook, ToolCallHookAction};
    type Model = rig::providers::openai::CompletionModel;
    let (id, mut gov, reqs, bindings) = fixture();
    gov.limits.as_mut().unwrap().max_tool_calls = Some(2);
    let entries =
        scoped_resource_contracts(&id, &gov, &reqs, &bindings).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let hook = ContractHook::with_contracts(tx, entries);
    for (tool, args) in [
        ("sheet_read", json!({"spreadsheet_id":"sheet-1"})),
        ("doc_read", json!({"document_id":"doc-1"})),
    ] {
        assert!(matches!(
            PromptHook::<Model>::on_tool_call(
                &hook,
                tool,
                None,
                "",
                &args.to_string()
            )
            .await,
            ToolCallHookAction::Continue
        ));
    }
    assert!(matches!(
        PromptHook::<Model>::on_tool_call(
            &hook,
            "doc_read",
            None,
            "",
            &json!({"document_id":"doc-1"}).to_string()
        )
        .await,
        ToolCallHookAction::Skip { .. }
    ));

    let entries =
        scoped_resource_contracts(&id, &gov, &reqs, &bindings).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let hook = ContractHook::with_contracts(tx, entries);
    assert!(matches!(
        PromptHook::<Model>::on_tool_call(
            &hook,
            "doc_read",
            None,
            "",
            &json!({"spreadsheet_id":"sheet-1"}).to_string()
        )
        .await,
        ToolCallHookAction::Skip { .. }
    ));
}

#[test]
fn external_connectors_cannot_claim_native_bypass_names() {
    let (id, gov, reqs, bindings) = fixture();
    for name in flowflow::application::tools::NOTES_TOOL_NAMES {
        let mut collision = reqs.clone();
        collision[0].manifest.tools[0].tool = name.into();
        collision[0].manifest.mcp_prefix = name.into();
        let error = scoped_resource_contracts(&id, &gov, &collision, &bindings)
            .unwrap_err();
        assert!(error.contains("claims native tool"), "{error}");
    }
}
