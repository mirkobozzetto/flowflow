use flowflow::application::agent_native::{
    validate_native_capabilities, NativeCapability,
};
use flowflow::domain::governance::{Approval, Mode, ToolPolicy};

fn policy(capability: NativeCapability) -> ToolPolicy {
    ToolPolicy {
        tool: capability.name().into(),
        mode: if capability.action().is_write() {
            Mode::ReadWrite
        } else {
            Mode::ReadOnly
        },
        approval: if capability.action().is_write() {
            Approval::RequireApproval
        } else {
            Approval::Auto
        },
        key_columns: None,
        max_calls_per_run: Some(2),
    }
}

#[test]
fn descriptors_match_all_actual_native_tool_names() {
    for name in flowflow::application::tools::NOTES_TOOL_NAMES {
        assert_eq!(NativeCapability::from_name(name).unwrap().name(), name);
    }
    assert!(NativeCapability::from_name("imaginary_native_tool").is_none());
}

#[test]
fn native_only_reads_need_neither_connectors_nor_a_write_channel() {
    let available = [
        NativeCapability::SearchNotes,
        NativeCapability::SummarizeFolder,
    ];
    let policies: Vec<_> = available.into_iter().map(policy).collect();
    let plan =
        validate_native_capabilities(&policies, &available, false).unwrap();
    assert_eq!(plan.policies().len(), 2);
    assert!(!plan.requires_write_approval());
    assert_eq!(plan.policies()[0].max_calls_per_run, Some(2));
}

#[test]
fn both_native_writes_require_explicit_consent_and_a_live_channel() {
    for capability in [
        NativeCapability::CreateNote,
        NativeCapability::ScheduleReminder,
    ] {
        let mut grant = policy(capability);
        assert!(validate_native_capabilities(
            &[grant.clone()],
            &[capability],
            false
        )
        .is_err());
        grant.approval = Approval::Auto;
        assert!(validate_native_capabilities(
            &[grant.clone()],
            &[capability],
            true
        )
        .is_err());
        grant.approval = Approval::RequireApproval;
        let plan = validate_native_capabilities(&[grant], &[capability], true)
            .unwrap();
        assert!(plan.requires_write_approval());
    }
}

#[test]
fn missing_mount_unknown_duplicate_and_incompatible_modes_fail_closed() {
    let capability = NativeCapability::SearchWeb;
    let grant = policy(capability);
    assert!(validate_native_capabilities(&[grant.clone()], &[], true).is_err());
    assert!(validate_native_capabilities(
        &[grant.clone(), grant],
        &[capability],
        true
    )
    .is_err());
    let mut unknown = policy(capability);
    unknown.tool = "remote_connector_tool".into();
    assert!(
        validate_native_capabilities(&[unknown], &[capability], true).is_err()
    );
    let mut write = policy(NativeCapability::CreateNote);
    write.mode = Mode::ReadOnly;
    assert!(validate_native_capabilities(
        &[write],
        &[NativeCapability::CreateNote],
        true
    )
    .is_err());
    let mut columns = policy(capability);
    columns.key_columns = Some(vec!["email".into()]);
    assert!(
        validate_native_capabilities(&[columns], &[capability], true).is_err()
    );
}
