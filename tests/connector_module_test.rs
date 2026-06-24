// M1.13 atomic module (issue #12): locks the device-pinned contract that `ContractHook` enforces
// for the google_sheets_list_spreadsheets proof. If the hardcoded manifest/governance drift so the
// gate stops Allowing the one tool, or starts Allowing another, this fails before a device run.

use flowflow::application::connector_module::{
    SHEETS_CONNECTOR_MANIFEST, SHEETS_LIST_ONLY_GOVERNANCE,
};
use flowflow::domain::governance::{
    gate, parse_connector_manifest, parse_governance, Decision, DenyReason,
    ProposedCall, RunState,
};
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
