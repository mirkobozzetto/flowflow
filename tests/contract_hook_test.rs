// The hook's suspended-write flow, exercised through the real PromptHook seam: approve
// continues the original call, reject/expire fail closed and abort the run, an illegal edit
// keeps the card pending, and the decision always comes from ANOTHER task (no lock is held
// across the await, or these tests deadlock).

use std::time::Duration;

use flowflow::application::approvals::{decide, UserDecision};
use flowflow::application::tools::{ContractHook, ProposalStatus, ToolEvent};
use flowflow::domain::governance::{
    parse_connector_manifest, parse_governance, ConnectorManifest, Governance,
};
use rig::agent::{PromptHook, ToolCallHookAction};
use serde_json::json;
use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

type Model = rig::providers::openai::CompletionModel;

fn sheets() -> ConnectorManifest {
    parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store",
          "server": "s", "mcp_prefix": "google_sheets_",
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

fn hook_with_events(
    timeout: Duration,
) -> (ContractHook, UnboundedReceiver<ToolEvent>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let hook = ContractHook::with_contract(tx, governed(), sheets())
        .with_approval_timeout(timeout);
    (hook, rx)
}

async fn satisfy_read(hook: &ContractHook) {
    let action = PromptHook::<Model>::on_tool_call(
        hook,
        "google_sheets_get_spreadsheet",
        None,
        "",
        &json!({"spreadsheet_id": "SHEET_A"}).to_string(),
    )
    .await;
    assert!(matches!(action, ToolCallHookAction::Continue));
}

async fn next_proposal_id(rx: &mut UnboundedReceiver<ToolEvent>) -> Uuid {
    loop {
        match rx.recv().await.expect("event") {
            ToolEvent::Proposal(view) => {
                return Uuid::parse_str(&view.id).unwrap()
            }
            _ => continue,
        }
    }
}

fn write_args() -> String {
    json!({"spreadsheet_id": "SHEET_A", "cell": "B2", "value": "devis vendredi"})
        .to_string()
}

#[tokio::test]
async fn approve_from_another_task_continues_the_call() {
    let (hook, mut rx) = hook_with_events(Duration::from_secs(5));
    satisfy_read(&hook).await;

    let h = hook.clone();
    let held = tokio::spawn(async move {
        PromptHook::<Model>::on_tool_call(
            &h,
            "google_sheets_write_to_cell",
            None,
            "",
            &write_args(),
        )
        .await
    });

    let id = next_proposal_id(&mut rx).await;
    decide(id, UserDecision::Approved).expect("pending");

    assert!(matches!(held.await.unwrap(), ToolCallHookAction::Continue));
    assert!(!hook.aborted());
    // Card frozen as Approved.
    let resolved = loop {
        match rx.recv().await.expect("event") {
            ToolEvent::ProposalResolved { status, .. } => break status,
            _ => continue,
        }
    };
    assert_eq!(resolved, ProposalStatus::Approved);
}

#[tokio::test]
async fn reject_skips_and_aborts_the_run() {
    let (hook, mut rx) = hook_with_events(Duration::from_secs(5));
    satisfy_read(&hook).await;

    let h = hook.clone();
    let held = tokio::spawn(async move {
        PromptHook::<Model>::on_tool_call(
            &h,
            "google_sheets_write_to_cell",
            None,
            "",
            &write_args(),
        )
        .await
    });

    let id = next_proposal_id(&mut rx).await;
    decide(id, UserDecision::Rejected).expect("pending");

    match held.await.unwrap() {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("rejected"), "reason: {reason}")
        }
        other => panic!("expected Skip, got {other:?}"),
    }
    assert!(hook.aborted(), "reject must end remaining write intents");
}

#[tokio::test]
async fn timeout_expires_fail_closed_and_aborts() {
    let (hook, mut rx) = hook_with_events(Duration::from_millis(60));
    satisfy_read(&hook).await;

    let h = hook.clone();
    let held = tokio::spawn(async move {
        PromptHook::<Model>::on_tool_call(
            &h,
            "google_sheets_write_to_cell",
            None,
            "",
            &write_args(),
        )
        .await
    });

    let _id = next_proposal_id(&mut rx).await;
    // Nobody decides.
    match held.await.unwrap() {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("expired"), "reason: {reason}")
        }
        other => panic!("expected Skip, got {other:?}"),
    }
    assert!(hook.aborted());
    let resolved = loop {
        match rx.recv().await.expect("event") {
            ToolEvent::ProposalResolved { status, .. } => break status,
            _ => continue,
        }
    };
    assert_eq!(resolved, ProposalStatus::Expired);
}

#[tokio::test]
async fn illegal_edit_keeps_the_card_pending_then_reject_ends_it() {
    let (hook, mut rx) = hook_with_events(Duration::from_secs(5));
    satisfy_read(&hook).await;

    let h = hook.clone();
    let held = tokio::spawn(async move {
        PromptHook::<Model>::on_tool_call(
            &h,
            "google_sheets_write_to_cell",
            None,
            "",
            &write_args(),
        )
        .await
    });

    let id = next_proposal_id(&mut rx).await;
    // Edit targeting a sibling sheet: out of bound_resource -> re-validation must refuse.
    decide(
        id,
        UserDecision::Edited(
            json!({"spreadsheet_id": "SHEET_B", "cell": "B2", "value": "x"}),
        ),
    )
    .expect("pending");

    let reason = loop {
        match rx.recv().await.expect("event") {
            ToolEvent::EditRejected { id: rid, reason } => {
                assert_eq!(rid, id.to_string());
                break reason;
            }
            _ => continue,
        }
    };
    assert!(reason.contains("bound"), "reason: {reason}");

    // The SAME card is still decidable: reject ends the flow.
    decide(id, UserDecision::Rejected).expect("card re-armed");
    assert!(matches!(
        held.await.unwrap(),
        ToolCallHookAction::Skip { .. }
    ));
    assert!(hook.aborted());
}

#[tokio::test]
async fn legal_edit_without_peer_fails_closed_with_report() {
    let (hook, mut rx) = hook_with_events(Duration::from_secs(5));
    satisfy_read(&hook).await;

    let h = hook.clone();
    let held = tokio::spawn(async move {
        PromptHook::<Model>::on_tool_call(
            &h,
            "google_sheets_write_to_cell",
            None,
            "",
            &write_args(),
        )
        .await
    });

    let id = next_proposal_id(&mut rx).await;
    decide(
        id,
        UserDecision::Edited(
            json!({"spreadsheet_id": "SHEET_A", "cell": "B2", "value": "devis JEUDI"}),
        ),
    )
    .expect("pending");

    // No MCP peer attached in this test: the edited call must fail CLOSED with a report,
    // never silently pretend it ran.
    match held.await.unwrap() {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("failed"), "reason: {reason}")
        }
        other => panic!("expected Skip, got {other:?}"),
    }
    let resolved = loop {
        match rx.recv().await.expect("event") {
            ToolEvent::ProposalResolved { status, .. } => break status,
            _ => continue,
        }
    };
    assert_eq!(resolved, ProposalStatus::Rejected);
}
