// Locks the shipped agent's contract end to end: the fixture verifies against the pinned key, builds
// into a sound, gate-validated agent, and the gate + chain still bite. Fails before a device run if
// the manifest, governance, or chain drift.

use flowflow::application::agent_builder::{build_agent, BuiltAgent};
use flowflow::application::connector_module::{
    bind_spreadsheet, current_bindings, parse_spreadsheets,
    python_literal_to_json, spreadsheet_id_from_url, title_from_meta,
    unbind_spreadsheet, FIXTURE_AGENT_ID, FIXTURE_PACKAGE,
};
use flowflow::domain::agent_manifest::{verify_package, ADMIN_PUBKEY};
use flowflow::domain::governance::{
    gate, validate_governance, Decision, DenyReason, ProposedCall, RunState,
};
use flowflow::infrastructure::persistence::Database;
use serde_json::json;
use tempfile::tempdir;

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

// The arm-time list parser tolerates the shapes a connector might return: a bare array, a wrapped
// array, loose id/name keys, and rows with no id (skipped). Exact ids matter - they become the bound.
#[test]
fn parse_spreadsheets_reads_a_bare_array() {
    let got = parse_spreadsheets(&json!([
        { "id": "1Ab", "name": "Clients" },
        { "id": "2Cd", "name": "Leads" }
    ]));
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].id, "1Ab");
    assert_eq!(got[0].name, "Clients");
}

#[test]
fn parse_spreadsheets_unwraps_and_picks_loose_keys() {
    let got = parse_spreadsheets(&json!({
        "spreadsheets": [{ "spreadsheetId": "1Ab", "title": "Clients" }]
    }));
    assert_eq!(
        got,
        vec![flowflow::application::connector_module::Spreadsheet {
            id: "1Ab".into(),
            name: "Clients".into(),
            modified_at: String::new(),
        }]
    );
}

#[test]
fn parse_spreadsheets_skips_rows_without_id_and_falls_back_to_id_for_name() {
    let got = parse_spreadsheets(&json!([
        { "name": "no id here" },
        { "id": "1Ab" }
    ]));
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, "1Ab");
    assert_eq!(got[0].name, "1Ab");
}

// The Sheets connector returns its list as a Python dict repr (str(), not json.dumps): single quotes,
// None. Normalize to JSON, then the existing walker reads it. Apostrophes inside names must survive.
#[test]
fn parse_spreadsheets_handles_python_repr_with_apostrophes() {
    // `name` with an apostrophe is double-quoted by Python repr; another uses an escaped single quote.
    let wire = r#"{'spreadsheets': [{'id': '1Ab', 'name': "L'Oreal clients", 'link': 'http://x', 'owners': ['Mirko']}, {'id': '2Cd', 'name': 'It\'s mine', 'modifiedAt': None}], 'total_count': 2}"#;
    let normalized = python_literal_to_json(wire);
    let value: serde_json::Value =
        serde_json::from_str(&normalized).expect("normalized parses as JSON");
    let got = parse_spreadsheets(&value);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].id, "1Ab");
    assert_eq!(got[0].name, "L'Oreal clients");
    assert_eq!(got[1].id, "2Cd");
    assert_eq!(got[1].name, "It's mine");
}

// The exact wire payload captured from the live Klavis server on device (4 sheets, two share a name).
#[test]
fn parse_spreadsheets_reads_the_real_device_payload() {
    let wire = r#"{'spreadsheets': [{'id': '1qsIYbq4Z3S7KsAgVUwsYkey4s7EcSNpL4meyJ2WJ7MQ', 'name': 'Suivi des Dépenses - Mois', 'createdAt': '2026-06-22T08:19:18.192Z', 'modifiedAt': '2026-06-22T08:19:18.584Z', 'link': 'https://docs.google.com/spreadsheets/d/1qsIYbq4Z3S7KsAgVUwsYkey4s7EcSNpL4meyJ2WJ7MQ/edit?usp=drivesdk', 'owners': ['Mirko Bozzetto']}, {'id': '1ufFuq946BdQw6hYRNsedXGL64lG1g0CHQyGxQQivSEQ', 'name': 'Suivi des Dépenses - Mois', 'createdAt': '2026-06-22T00:14:18.810Z', 'modifiedAt': '2026-06-22T00:14:19.253Z', 'link': 'https://docs.google.com/spreadsheets/d/1ufFuq946BdQw6hYRNsedXGL64lG1g0CHQyGxQQivSEQ/edit?usp=drivesdk', 'owners': ['Mirko Bozzetto']}, {'id': '1UHWXiaSwjUJEa6GSKRiusvv9blEgPJroeua65RyYa-Q', 'name': 'CRM Test', 'createdAt': '2026-06-21T18:46:53.990Z', 'modifiedAt': '2026-06-21T18:46:54.397Z', 'link': 'https://docs.google.com/spreadsheets/d/1UHWXiaSwjUJEa6GSKRiusvv9blEgPJroeua65RyYa-Q/edit?usp=drivesdk', 'owners': ['Mirko Bozzetto']}, {'id': '18CtPAf3bjx-EijVG7zi2TxkU2MUOdrI1lz3vDJDJXzg', 'name': 'CRM Prospects', 'createdAt': '2026-06-21T18:00:12.303Z', 'modifiedAt': '2026-06-21T18:04:12.135Z', 'link': 'https://docs.google.com/spreadsheets/d/18CtPAf3bjx-EijVG7zi2TxkU2MUOdrI1lz3vDJDJXzg/edit?usp=drivesdk', 'owners': ['Mirko Bozzetto']}], 'total_count': 4}"#;
    let value: serde_json::Value =
        serde_json::from_str(&python_literal_to_json(wire))
            .expect("real payload normalizes to JSON");
    let got = parse_spreadsheets(&value);
    assert_eq!(got.len(), 4);
    assert_eq!(got[0].id, "1qsIYbq4Z3S7KsAgVUwsYkey4s7EcSNpL4meyJ2WJ7MQ");
    assert_eq!(got[0].name, "Suivi des Dépenses - Mois");
    assert_eq!(got[3].name, "CRM Prospects");
}

#[test]
fn python_literal_to_json_is_identity_on_valid_json() {
    let json = r#"{"spreadsheets":[{"id":"1Ab","name":"Q3"}],"total_count":1}"#;
    let value: serde_json::Value =
        serde_json::from_str(&python_literal_to_json(json))
            .expect("valid json survives normalization");
    assert_eq!(parse_spreadsheets(&value)[0].id, "1Ab");
}

// Bind-by-URL: the id is the `<id>` in `/spreadsheets/d/<id>/`, the inverse of the UI's sheet_url.
#[test]
fn spreadsheet_id_from_url_parses_real_and_bare_urls() {
    let drive = "https://docs.google.com/spreadsheets/d/1qsIYbq4Z3S7KsAgVUwsYkey4s7EcSNpL4meyJ2WJ7MQ/edit?usp=drivesdk";
    assert_eq!(
        spreadsheet_id_from_url(drive).as_deref(),
        Some("1qsIYbq4Z3S7KsAgVUwsYkey4s7EcSNpL4meyJ2WJ7MQ")
    );
    assert_eq!(
        spreadsheet_id_from_url(
            "https://docs.google.com/spreadsheets/d/1Ab/edit"
        )
        .as_deref(),
        Some("1Ab")
    );
    assert_eq!(
        spreadsheet_id_from_url("https://docs.google.com/spreadsheets/d/1Ab")
            .as_deref(),
        Some("1Ab")
    );
}

// Multi-bind app layer: adding is additive and id-deduped (name refreshes), removing drops one id, and
// removing the last clears the binding to the manifest placeholder (None).
#[test]
fn multi_bind_adds_dedups_and_removes() {
    let dir = tempdir().unwrap();
    let db = Database::open_at(dir.path().join("t.db")).unwrap();

    bind_spreadsheet(&db, "A", "Alpha").unwrap();
    bind_spreadsheet(&db, "B", "Beta").unwrap();
    bind_spreadsheet(&db, "A", "Alpha again").unwrap();
    let mut got = current_bindings(&db);
    got.sort();
    assert_eq!(
        got,
        vec![
            ("A".to_string(), "Alpha again".to_string()),
            ("B".to_string(), "Beta".to_string()),
        ]
    );

    unbind_spreadsheet(&db, "A").unwrap();
    assert_eq!(
        current_bindings(&db),
        vec![("B".to_string(), "Beta".to_string())]
    );

    unbind_spreadsheet(&db, "B").unwrap();
    assert!(current_bindings(&db).is_empty());
    assert!(db.get_agent_binding(FIXTURE_AGENT_ID).is_none());
}

// list_sheets returns the spreadsheet title at the top level, as a Python dict repr (Klavis str()).
// Validates name resolution parses the real shape - the title comes out, not the id.
#[test]
fn list_sheets_title_extracted_from_python_repr() {
    let wire = r#"{'spreadsheetId': '1tJDSM5ki0', 'url': 'https://docs.google.com/spreadsheets/d/1tJDSM5ki0/edit', 'title': "L'Oreal CRM Prospects", 'sheets': [{'sheetId': 0, 'title': 'Feuille 1', 'index': 0, 'rowCount': 1000, 'columnCount': 26}]}"#;
    let v: serde_json::Value =
        serde_json::from_str(&python_literal_to_json(wire))
            .expect("list_sheets repr normalizes to JSON");
    assert_eq!(
        title_from_meta(&v).as_deref(),
        Some("L'Oreal CRM Prospects")
    );
}

#[test]
fn title_from_meta_is_none_when_absent_or_empty() {
    assert_eq!(title_from_meta(&json!({ "spreadsheetId": "x" })), None);
    assert_eq!(title_from_meta(&json!({ "title": "" })), None);
}

#[test]
fn spreadsheet_id_from_url_rejects_non_urls() {
    assert_eq!(spreadsheet_id_from_url("not a url"), None);
    assert_eq!(spreadsheet_id_from_url(""), None);
    assert_eq!(
        spreadsheet_id_from_url("https://docs.google.com/spreadsheets/d//edit"),
        None
    );
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
