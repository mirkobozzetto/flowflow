// Locks the shipped agent's contract end to end: the fixture verifies against the pinned key, builds
// into a sound, gate-validated agent, and the gate + chain still bite. Fails before a device run if
// the manifest, governance, or chain drift.

use flowflow::application::agent_builder::{build_agent, BuiltAgent};
use flowflow::application::connector_module::{
    FIXTURE_AGENT_ID, FIXTURE_PACKAGE,
};
use flowflow::domain::agent_manifest::{verify_package, ADMIN_PUBKEY};
use flowflow::domain::governance::{
    gate, validate_governance, Decision, DenyReason, ProposedCall, RunState,
};
use serde_json::json;

fn built() -> BuiltAgent {
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY)
        .expect("fixture verifies");
    build_agent(&verified.manifest).expect("fixture builds")
}

#[test]
fn fixture_verifies_and_identifies() {
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap();
    assert_eq!(verified.manifest.id, FIXTURE_AGENT_ID);
    assert!(verified.content_digest.starts_with("sha256:"));
}

#[test]
fn built_governance_is_install_valid() {
    let b = built();
    validate_governance(&b.governance, &b.connector)
        .expect("built governance passes install validation");
}

#[test]
fn gate_allows_list_and_bounded_read_write() {
    let b = built();
    let mut run = RunState::default();

    let list = ProposedCall::new("google_sheets_list_spreadsheets", json!({}));
    assert!(gate(&b.governance, &b.connector, &list, &mut run).is_allowed());

    // read then write, both on the bound resource: the read satisfies read_before_write.
    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "bound-at-install" }),
    );
    assert!(gate(&b.governance, &b.connector, &read, &mut run).is_allowed());
    let write = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "bound-at-install", "cell": "A1", "value": "v" }),
    );
    assert!(gate(&b.governance, &b.connector, &write, &mut run).is_allowed());
}

#[test]
fn gate_denies_off_bound_read() {
    let b = built();
    let mut run = RunState::default();
    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "some-other-sheet" }),
    );
    assert!(matches!(
        gate(&b.governance, &b.connector, &read, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource { .. })
    ));
}

#[test]
fn gate_denies_ungoverned_tool() {
    let b = built();
    let mut run = RunState::default();
    let call = ProposedCall::new("google_sheets_create_spreadsheet", json!({}));
    assert!(matches!(
        gate(&b.governance, &b.connector, &call, &mut run),
        Decision::Deny(DenyReason::NotAllowed { .. })
    ));
}

#[test]
fn sync_chain_is_sound() {
    let b = built();
    let chain = b.chains.get("sync").expect("sync chain present");
    chain.validate().expect("chain is sound");
    assert_eq!(chain.initial, "find");
    assert_eq!(
        chain.state("find").unwrap().on_done.as_deref(),
        Some("read")
    );
    assert!(chain.state("answer").unwrap().terminal);
}

// Layer 1 (gate) and Layer 2 (chain state filter) are independent: a governed tool is still refused by
// a state that does not list it. `get_spreadsheet` clears the gate but is not in `find`.
#[test]
fn state_filter_is_independent_of_the_gate() {
    let b = built();
    let chain = b.chains.get("sync").unwrap();
    let mut run = RunState::default();
    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "bound-at-install" }),
    );
    assert!(gate(&b.governance, &b.connector, &read, &mut run).is_allowed());
    assert!(!chain
        .state("find")
        .unwrap()
        .permits("google_sheets_get_spreadsheet"));
    assert!(chain
        .state("read")
        .unwrap()
        .permits("google_sheets_get_spreadsheet"));
}
