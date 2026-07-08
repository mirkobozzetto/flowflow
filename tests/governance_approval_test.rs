// The approval floor: a require_approval write that every other check allows is HELD, commits
// nothing, and executes only by consuming a one-shot fingerprint grant. Edited args are a new
// fingerprint, so a stale approval can never authorize different bytes.

use flowflow::domain::governance::{
    call_fingerprint, gate, parse_connector_manifest, parse_governance,
    ConnectorManifest, Decision, DenyReason, Governance, ProposedCall,
    RunState,
};
use serde_json::json;

fn sheets() -> ConnectorManifest {
    parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store",
          "server": "ghcr.io/klavis-ai/google-sheets-mcp-server", "mcp_prefix": "google_sheets_",
          "provides": ["search", "read", "update"],
          "tools": [
            { "tool": "google_sheets_get_spreadsheet", "resource": "spreadsheet", "action": "read",   "risk": "read_only" },
            { "tool": "google_sheets_write_to_cell",   "resource": "cell",        "action": "update", "risk": "read_write" }
          ]
        }"#,
    )
    .unwrap()
}

fn governed() -> Governance {
    parse_governance(
        r#"{
          "tools": [
            { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" },
            { "tool": "google_sheets_write_to_cell",   "mode": "read_write", "approval": "require_approval" }
          ],
          "bound_resource": { "spreadsheet_id": "SHEET_A" },
          "read_before_write": true,
          "deny_destructive": true
        }"#,
    )
    .unwrap()
}

fn read_call() -> ProposedCall {
    ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({"spreadsheet_id": "SHEET_A"}),
    )
}

fn write_call() -> ProposedCall {
    ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({"spreadsheet_id": "SHEET_A", "cell": "B2", "value": "devis vendredi"}),
    )
}

fn run_after_read(gov: &Governance, conn: &ConnectorManifest) -> RunState {
    let mut run = RunState::default();
    assert!(gate(gov, conn, &read_call(), &mut run).is_allowed());
    run
}

#[test]
fn legal_require_approval_write_holds_and_commits_nothing() {
    let (gov, conn) = (governed(), sheets());
    let mut run = run_after_read(&gov, &conn);
    let before = run.clone();

    let call = write_call();
    match gate(&gov, &conn, &call, &mut run) {
        Decision::Hold(p) => {
            assert_eq!(p.tool, call.tool);
            assert_eq!(p.args, call.args);
            assert_eq!(p.fingerprint, call_fingerprint(&call.tool, &call.args));
        }
        other => panic!("expected Hold, got {other:?}"),
    }
    // Hold mutates NO run accounting.
    assert_eq!(run.tool_calls, before.tool_calls);
    assert_eq!(run.per_tool, before.per_tool);
    assert_eq!(run.read_resources, before.read_resources);
    assert!(run.approvals.is_empty());
}

#[test]
fn approved_fingerprint_is_consumed_exactly_once() {
    let (gov, conn) = (governed(), sheets());
    let mut run = run_after_read(&gov, &conn);
    let call = write_call();

    run.approvals
        .insert(call_fingerprint(&call.tool, &call.args));
    let calls_before = run.tool_calls;

    assert!(gate(&gov, &conn, &call, &mut run).is_allowed());
    assert!(run.approvals.is_empty(), "grant must be consumed");
    assert_eq!(run.tool_calls, calls_before + 1, "Allow commits budgets");

    // The same call again is a NEW proposal: one approval = one execution.
    assert!(matches!(
        gate(&gov, &conn, &call, &mut run),
        Decision::Hold(_)
    ));
}

#[test]
fn edited_args_are_a_new_fingerprint() {
    let (gov, conn) = (governed(), sheets());
    let mut run = run_after_read(&gov, &conn);

    let original = write_call();
    run.approvals
        .insert(call_fingerprint(&original.tool, &original.args));

    let edited = ProposedCall::new(
        original.tool.clone(),
        json!({"spreadsheet_id": "SHEET_A", "cell": "B2", "value": "devis JEUDI"}),
    );
    assert!(
        matches!(gate(&gov, &conn, &edited, &mut run), Decision::Hold(_)),
        "an approval of other bytes must not authorize the edit"
    );
    // The original grant is untouched by the held edited call.
    assert_eq!(run.approvals.len(), 1);
}

#[test]
fn fingerprint_is_key_order_independent() {
    let a = call_fingerprint(
        "t",
        &json!({"cell": "B2", "spreadsheet_id": "SHEET_A"}),
    );
    let b = call_fingerprint(
        "t",
        &json!({"spreadsheet_id": "SHEET_A", "cell": "B2"}),
    );
    assert_eq!(a, b);
}

#[test]
fn require_approval_read_stays_direct() {
    let mut gov = governed();
    // Force approval on the read tool too: is_write() keeps reads direct.
    for tp in &mut gov.tools {
        tp.approval = flowflow::domain::governance::Approval::RequireApproval;
    }
    let conn = sheets();
    let mut run = RunState::default();
    assert!(gate(&gov, &conn, &read_call(), &mut run).is_allowed());
}

#[test]
fn illegal_write_denies_before_any_hold() {
    let (gov, conn) = (governed(), sheets());
    let mut run = RunState::default(); // nothing read yet
    assert!(matches!(
        gate(&gov, &conn, &write_call(), &mut run),
        Decision::Deny(DenyReason::ReadBeforeWrite { .. })
    ));
}

#[test]
fn auto_and_absent_approval_write_directly() {
    let (mut gov, conn) = (governed(), sheets());
    gov.tools[1].approval = flowflow::domain::governance::Approval::Auto;
    let mut run = run_after_read(&gov, &conn);
    assert!(gate(&gov, &conn, &write_call(), &mut run).is_allowed());
}
