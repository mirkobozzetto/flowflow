// Locks the pinned contract: the gate must Allow only google_sheets_list_spreadsheets and Deny
// anything else. Fails before a device run if the hardcoded manifest/governance drift.

use flowflow::application::connector_module::{
    SHEETS_CONNECTOR_MANIFEST, SHEETS_LIST_ONLY_GOVERNANCE, SHEETS_SYNC_CHAIN,
    SHEETS_SYNC_GOVERNANCE,
};
use flowflow::domain::governance::{
    gate, parse_connector_manifest, parse_governance, validate_governance,
    Decision, DenyReason, ProposedCall, RunState,
};
use flowflow::domain::orchestration::parse_chain;
use serde_json::json;

#[test]
fn pinned_contract_parses() {
    parse_connector_manifest(SHEETS_CONNECTOR_MANIFEST)
        .expect("connector manifest parses");
    parse_governance(SHEETS_LIST_ONLY_GOVERNANCE).expect("governance parses");
}

#[test]
fn gate_allows_list_spreadsheets() {
    let conn = parse_connector_manifest(SHEETS_CONNECTOR_MANIFEST).unwrap();
    let gov = parse_governance(SHEETS_LIST_ONLY_GOVERNANCE).unwrap();
    let call = ProposedCall::new("google_sheets_list_spreadsheets", json!({}));
    let mut run = RunState::default();
    assert!(matches!(
        gate(&gov, &conn, &call, &mut run),
        Decision::Allow
    ));
}

#[test]
fn gate_denies_any_other_tool() {
    let conn = parse_connector_manifest(SHEETS_CONNECTOR_MANIFEST).unwrap();
    let gov = parse_governance(SHEETS_LIST_ONLY_GOVERNANCE).unwrap();

    // A read tool that lives in the manifest but is NOT in the one-tool allowlist.
    let read = ProposedCall::new("google_sheets_get_spreadsheet", json!({}));
    let mut run = RunState::default();
    assert!(matches!(
        gate(&gov, &conn, &read, &mut run),
        Decision::Deny(DenyReason::NotAllowed { .. })
    ));

    // A write tool, likewise outside the allowlist.
    let write = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "X", "cell": "A1", "value": "v" }),
    );
    let mut run = RunState::default();
    assert!(matches!(
        gate(&gov, &conn, &write, &mut run),
        Decision::Deny(DenyReason::NotAllowed { .. })
    ));
}

#[test]
fn pinned_chain_parses_and_validates() {
    let chain = parse_chain(SHEETS_SYNC_CHAIN).expect("chain parses");
    chain.validate().expect("chain is sound");
    assert_eq!(chain.initial, "find");
    assert_eq!(
        chain.state("find").unwrap().on_done.as_deref(),
        Some("read")
    );
    assert_eq!(
        chain.state("act").unwrap().on_done.as_deref(),
        Some("answer")
    );
    assert!(chain.state("answer").unwrap().terminal);
}

#[test]
fn pinned_sync_governance_is_install_valid() {
    let conn = parse_connector_manifest(SHEETS_CONNECTOR_MANIFEST).unwrap();
    let gov = parse_governance(SHEETS_SYNC_GOVERNANCE).unwrap();
    validate_governance(&gov, &conn)
        .expect("pinned sync governance passes install validation");
}

#[test]
fn sync_governance_allows_every_chain_tool() {
    let conn = parse_connector_manifest(SHEETS_CONNECTOR_MANIFEST).unwrap();
    let gov = parse_governance(SHEETS_SYNC_GOVERNANCE).unwrap();

    let list = ProposedCall::new("google_sheets_list_spreadsheets", json!({}));
    let mut run = RunState::default();
    assert!(gate(&gov, &conn, &list, &mut run).is_allowed());

    // read then write, both targeting the pinned resource: the read satisfies read_before_write and the
    // bounded write clears. A write to a different sheet would be refused (off-bound).
    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "bound-at-install" }),
    );
    assert!(gate(&gov, &conn, &read, &mut run).is_allowed());
    let write = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "bound-at-install", "cell": "A1", "value": "v" }),
    );
    assert!(gate(&gov, &conn, &write, &mut run).is_allowed());
}

// Layer 1 (governance gate) and Layer 2 (chain state filter) are independent: a tool the governance allows
// is still refused by a state that does not list it. `get_spreadsheet` clears the gate but is not in the
// `find` state's allowed_tools.
#[test]
fn state_filter_is_independent_of_the_gate() {
    let conn = parse_connector_manifest(SHEETS_CONNECTOR_MANIFEST).unwrap();
    let gov = parse_governance(SHEETS_SYNC_GOVERNANCE).unwrap();
    let chain = parse_chain(SHEETS_SYNC_CHAIN).unwrap();

    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "bound-at-install" }),
    );
    let mut run = RunState::default();
    // Layer 1 allows the read.
    assert!(gate(&gov, &conn, &read, &mut run).is_allowed());
    // Layer 2: the `find` state does not expose it, the `read` state does.
    assert!(!chain
        .state("find")
        .unwrap()
        .permits("google_sheets_get_spreadsheet"));
    assert!(chain
        .state("read")
        .unwrap()
        .permits("google_sheets_get_spreadsheet"));
}
