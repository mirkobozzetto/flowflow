// The pending-approval registry: decide resolves the awaiting hook, timeouts fail closed,
// touch extends a live card, and a dropped registration can never leak a decidable entry.

use std::time::Duration;

use flowflow::application::approvals::{
    await_decision, decide, register, touch, view_for, DecideError, Outcome,
    UserDecision,
};
use flowflow::domain::governance::{
    call_fingerprint, parse_connector_manifest, Action, Proposal,
};
use serde_json::json;

#[tokio::test]
async fn approve_resolves_the_await() {
    let reg = register(Duration::from_secs(5));
    let id = reg.id();
    let waiter = tokio::spawn(await_decision(reg));
    tokio::task::yield_now().await;

    decide(id, UserDecision::Approved).expect("pending");
    assert_eq!(waiter.await.unwrap(), Outcome::Approved);
}

#[tokio::test]
async fn edited_args_travel_through() {
    let reg = register(Duration::from_secs(5));
    let id = reg.id();
    let waiter = tokio::spawn(await_decision(reg));
    tokio::task::yield_now().await;

    let edited = json!({"cell": "B2", "value": "corrig\u{00e9}"});
    decide(id, UserDecision::Edited(edited.clone())).expect("pending");
    assert_eq!(waiter.await.unwrap(), Outcome::Edited(edited));
}

#[tokio::test]
async fn timeout_fails_closed_and_kills_the_entry() {
    let reg = register(Duration::from_millis(50));
    let id = reg.id();
    assert_eq!(await_decision(reg).await, Outcome::Expired);
    // The card deciding after expiry gets Expired, never a silent success.
    assert_eq!(
        decide(id, UserDecision::Approved),
        Err(DecideError::Expired)
    );
}

#[tokio::test]
async fn touch_extends_a_live_card() {
    let reg = register(Duration::from_millis(80));
    let id = reg.id();
    let waiter = tokio::spawn(await_decision(reg));

    // Keep touching past the original deadline, then decide: the card must still be live.
    for _ in 0..4 {
        tokio::time::sleep(Duration::from_millis(40)).await;
        touch(id, Duration::from_millis(80));
    }
    decide(id, UserDecision::Rejected).expect("touch kept it pending");
    assert_eq!(waiter.await.unwrap(), Outcome::Rejected);
}

#[tokio::test]
async fn dropped_registration_cannot_be_decided() {
    let reg = register(Duration::from_secs(5));
    let id = reg.id();
    drop(reg); // run future cancelled
    assert_eq!(
        decide(id, UserDecision::Approved),
        Err(DecideError::Expired)
    );
}

#[test]
fn view_orders_sheets_fields_first_and_keeps_the_rest() {
    let conn = parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store",
          "server": "s", "mcp_prefix": "google_sheets_",
          "provides": ["update"],
          "tools": [
            { "tool": "google_sheets_write_to_cell", "resource": "cell", "action": "update", "risk": "read_write" }
          ]
        }"#,
    )
    .unwrap();
    let args = json!({
        "value": "devis vendredi",
        "spreadsheet_id": "SHEET_A",
        "cell": "B2",
        "extra_flag": true
    });
    let proposal = Proposal {
        tool: "google_sheets_write_to_cell".into(),
        action: Action::Update,
        fingerprint: call_fingerprint("google_sheets_write_to_cell", &args),
        args,
    };
    let view = view_for(uuid::Uuid::new_v4(), &proposal, &conn);

    assert_eq!(view.action, "update");
    let keys: Vec<&str> = view.rows.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, vec!["spreadsheet_id", "cell", "value", "extra_flag"]);
    assert_eq!(view.rows[1].1, "B2");
    assert_eq!(view.rows[2].1, "devis vendredi");
}

#[test]
fn view_falls_back_to_raw_args_for_unknown_connectors() {
    let conn = parse_connector_manifest(
        r#"{
          "connector": "other", "type": "other_store",
          "server": "s", "mcp_prefix": "x_",
          "provides": ["append"],
          "tools": [
            { "tool": "x_append_row", "resource": "row", "action": "append", "risk": "read_write" }
          ]
        }"#,
    )
    .unwrap();
    let args = json!({"payload": {"a": 1}});
    let proposal = Proposal {
        tool: "x_append_row".into(),
        action: Action::Append,
        fingerprint: call_fingerprint("x_append_row", &args),
        args,
    };
    let view = view_for(uuid::Uuid::new_v4(), &proposal, &conn);
    assert_eq!(view.action, "append");
    assert_eq!(view.rows.len(), 1);
    assert_eq!(view.rows[0].0, "payload");
}
