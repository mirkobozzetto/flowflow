// Locks the shipped agent's contract end to end: the fixture verifies against the pinned key, builds
// into a sound, gate-validated agent, and the gate + chain still bite. Fails before a device run if
// the manifest, governance, or chain drift.

use flowflow::application::agent_builder::{build_agent, BuiltAgent};
use flowflow::application::chain::{ChainOutcome, ChainStep};
use flowflow::application::connector_module::{
    armed_schema_for, bind_spreadsheet, current_bindings, format_outcome,
    parse_spreadsheets, python_literal_to_json, sheet_titles_from_meta,
    spreadsheet_id_from_url, title_from_meta, unbind_spreadsheet,
    validate_headers, FIXTURE_AGENT_ID, FIXTURE_PACKAGE,
};
use flowflow::domain::agent_manifest::{verify_package, ADMIN_PUBKEY};
use flowflow::domain::governance::{
    gate, validate_governance, Decision, DenyReason, ProposedCall, RunState,
};
use flowflow::infrastructure::persistence::installed_connector_repo::SHEETS_BACKFILL_MANIFEST_JSON;
use flowflow::infrastructure::persistence::Database;
use serde_json::json;
use tempfile::tempdir;

fn built() -> BuiltAgent {
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY)
        .expect("fixture verifies");
    build_agent(&verified.manifest, "google", SHEETS_BACKFILL_MANIFEST_JSON)
        .expect("fixture builds")
}

// The golden fixture keeps its historical id; the arm path now targets FIXTURE_AGENT_ID,
// so tests that exercise binding re-key the pinned row to it after install.
fn install_arm_agent(db: &Database) {
    db.install_agent(&verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap())
        .unwrap();
    db.conn()
        .execute("UPDATE installed_agents SET id = ?1", [FIXTURE_AGENT_ID])
        .unwrap();
}

#[test]
fn fixture_verifies_and_identifies() {
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap();
    assert_eq!(verified.manifest.id, "agent-crm-sync");
    assert!(verified.content_digest.starts_with("sha256:"));
}

#[test]
fn built_governance_is_install_valid() {
    let b = built();
    validate_governance(&b.governance, &b.connector())
        .expect("built governance passes install validation");
}

#[test]
fn gate_scopes_unfilterable_list_and_bounds_read_write() {
    let b = built();
    let mut run = RunState::default();

    // The backfill manifest declares no `scoping`, so with a pinned bound (even the
    // install placeholder) an unfilterable listing is refused: its response would leak
    // every resource. The manifest's scoping block re-enables it, hook-filtered.
    let list = ProposedCall::new("google_sheets_list_spreadsheets", json!({}));
    assert!(matches!(
        gate(&b.governance, &b.connector(), &list, &mut run),
        Decision::Deny(DenyReason::UnscopedSearch { .. })
    ));

    // read then write, both on the bound resource: the read satisfies read_before_write.
    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "bound-at-install" }),
    );
    assert!(gate(&b.governance, &b.connector(), &read, &mut run).is_allowed());
    let write = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "bound-at-install", "cell": "A1", "value": "v" }),
    );
    assert!(gate(&b.governance, &b.connector(), &write, &mut run).is_allowed());
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
        gate(&b.governance, &b.connector(), &read, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource { .. })
    ));
}

#[test]
fn gate_denies_ungoverned_tool() {
    let b = built();
    let mut run = RunState::default();
    let call = ProposedCall::new("google_sheets_create_spreadsheet", json!({}));
    assert!(matches!(
        gate(&b.governance, &b.connector(), &call, &mut run),
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
#[tokio::test]
async fn multi_bind_adds_dedups_and_removes() {
    let dir = tempdir().unwrap();
    let db = Database::open_at(dir.path().join("t.db")).unwrap();
    // Bind targets the pinned agent row; install it (the runtime install now comes from the backend).
    install_arm_agent(&db);

    // No backend configured: schema capture is unavailable and arming still succeeds.
    bind_spreadsheet(&db, "A", "Alpha").await.unwrap();
    bind_spreadsheet(&db, "B", "Beta").await.unwrap();
    bind_spreadsheet(&db, "A", "Alpha again").await.unwrap();
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

// ---- RFC 0023: the single binding reader (scalar / v1 object / v2 array) + v2-only-if-pin writer ----

#[test]
fn parse_binding_reads_all_three_formats() {
    use flowflow::application::connector_module::{
        parse_binding, BindingEntry,
    };

    // Legacy scalar (pre-RFC 0011 single bind).
    assert_eq!(
        parse_binding(&json!("1Ab")),
        vec![BindingEntry {
            spreadsheet_id: "1Ab".into(),
            sheet: None
        }]
    );
    // v1 object, both the array form and the single-id form.
    assert_eq!(
        parse_binding(&json!({ "spreadsheet_id": ["A", "B"] })),
        vec![
            BindingEntry {
                spreadsheet_id: "A".into(),
                sheet: None
            },
            BindingEntry {
                spreadsheet_id: "B".into(),
                sheet: None
            }
        ]
    );
    assert_eq!(
        parse_binding(&json!({ "spreadsheet_id": "A" })),
        vec![BindingEntry {
            spreadsheet_id: "A".into(),
            sheet: None
        }]
    );
    // v2 array with and without a tab pin.
    assert_eq!(
        parse_binding(
            &json!([{ "spreadsheet_id": "A", "sheet": "Clients" }, { "spreadsheet_id": "B" }])
        ),
        vec![
            BindingEntry {
                spreadsheet_id: "A".into(),
                sheet: Some("Clients".into())
            },
            BindingEntry {
                spreadsheet_id: "B".into(),
                sheet: None
            }
        ]
    );
    // Garbage reads as unarmed (the gate fails closed on its own copy).
    assert!(parse_binding(&json!(42)).is_empty());
    assert!(parse_binding(&json!([{ "sheet": "NoId" }])).is_empty());
}

#[tokio::test]
async fn binding_written_v1_without_pin_and_v2_with_pin() {
    use flowflow::application::connector_module::{
        binding_entries, write_binding, BindingEntry,
    };
    let dir = tempdir().unwrap();
    let db = Database::open_at(dir.path().join("t.db")).unwrap();
    install_arm_agent(&db);

    // No tab pinned: the stored shape stays v1 (survives downgrade/old-code restore).
    write_binding(
        &db,
        &[
            BindingEntry {
                spreadsheet_id: "A".into(),
                sheet: None,
            },
            BindingEntry {
                spreadsheet_id: "B".into(),
                sheet: None,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        db.get_agent_binding(FIXTURE_AGENT_ID).unwrap(),
        json!({ "spreadsheet_id": ["A", "B"] })
    );

    // One pin: the whole binding moves to v2.
    write_binding(
        &db,
        &[
            BindingEntry {
                spreadsheet_id: "A".into(),
                sheet: Some("Clients".into()),
            },
            BindingEntry {
                spreadsheet_id: "B".into(),
                sheet: None,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        db.get_agent_binding(FIXTURE_AGENT_ID).unwrap(),
        json!([
            { "spreadsheet_id": "A", "sheet": "Clients" },
            { "spreadsheet_id": "B" }
        ])
    );
    // The reader round-trips it.
    assert_eq!(binding_entries(&db).len(), 2);

    // Arming another spreadsheet PRESERVES the existing tab pin.
    bind_spreadsheet(&db, "C", "Gamma").await.unwrap();
    let entries = binding_entries(&db);
    assert!(entries
        .iter()
        .any(|e| e.spreadsheet_id == "A"
            && e.sheet.as_deref() == Some("Clients")));
    assert!(entries
        .iter()
        .any(|e| e.spreadsheet_id == "C" && e.sheet.is_none()));

    // Disarming a pinned spreadsheet clears its pin with it.
    unbind_spreadsheet(&db, "A").unwrap();
    assert!(binding_entries(&db).iter().all(|e| e.sheet.is_none()));
}

#[test]
fn device_binding_wins_over_the_manifest_placeholder() {
    use flowflow::application::agent_builder::merge_bound;

    // v1 object binding: per-key override (existing behavior).
    let mut target = Some(json!({ "spreadsheet_id": "bound-at-install" }));
    merge_bound(&mut target, json!({ "spreadsheet_id": ["A", "B"] }));
    assert_eq!(target.unwrap(), json!({ "spreadsheet_id": ["A", "B"] }));

    // v2 array binding: each entry inherits manifest fields, its own fields win.
    let mut target =
        Some(json!({ "spreadsheet_id": "bound-at-install", "locked": "yes" }));
    merge_bound(
        &mut target,
        json!([{ "spreadsheet_id": "A", "sheet": "Clients" }]),
    );
    assert_eq!(
        target.unwrap(),
        json!([{ "spreadsheet_id": "A", "sheet": "Clients", "locked": "yes" }])
    );
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
    assert!(gate(&b.governance, &b.connector(), &read, &mut run).is_allowed());
    assert!(!chain
        .state("find")
        .unwrap()
        .permits("google_sheets_get_spreadsheet"));
    assert!(chain
        .state("read")
        .unwrap()
        .permits("google_sheets_get_spreadsheet"));
}

fn step(state: &str, outcome: &str, tools: &[&str]) -> ChainStep {
    ChainStep {
        state: state.into(),
        outcome: outcome.into(),
        tools: tools.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn format_outcome_renders_answer_plus_tool_footer_never_json() {
    let outcome = ChainOutcome {
        final_text: "Ta feuille contient 3 prospects.".into(),
        trace: vec![
            step("find", "narrated {'spreadsheets': [huge json]}", &[
                "called google_sheets_list_spreadsheets {} -> allowed",
                "result google_sheets_list_spreadsheets: {'spreadsheets': [huge json]}",
            ]),
            step("read", "narration", &[
                "called google_sheets_get_spreadsheet {\"id\":\"x\"} -> allowed",
                "result google_sheets_get_spreadsheet: {rows}",
            ]),
            step("answer", "Ta feuille contient 3 prospects.", &[]),
        ],
    };
    let out = format_outcome(&outcome);
    assert!(out.starts_with("Ta feuille contient 3 prospects."));
    assert!(out.contains("google_sheets_list_spreadsheets"));
    assert!(out.contains("google_sheets_get_spreadsheet"));
    assert!(
        !out.contains("huge json"),
        "payloads never reach the bubble"
    );
    assert!(
        !out.contains("result "),
        "raw log lines never reach the bubble"
    );
    assert!(
        !out.contains("[find]"),
        "state markers never reach the bubble"
    );
}

#[test]
fn format_outcome_surfaces_block_reason_on_early_break() {
    let outcome = ChainOutcome {
        // an early break never reached the terminal synthesis: final_text = raw transcript
        final_text: "[find] narrated stuff".into(),
        trace: vec![
            step(
                "find",
                "narrated stuff",
                &["called google_sheets_list_spreadsheets {} -> allowed"],
            ),
            step("act", "blocked: no servable tool", &[]),
        ],
    };
    let out = format_outcome(&outcome);
    assert!(out.starts_with("blocked: no servable tool"));
    assert!(!out.contains("[find]"));
}

#[test]
fn format_outcome_skipped_write_state_still_answers() {
    // A read-only question through a linear chain: the guarded write state is skipped
    // (no write possible), the terminal answer still reaches the bubble.
    let outcome = ChainOutcome {
        final_text: "Ta feuille contient 3 prospects.".into(),
        trace: vec![
            step("find", "narration", &["called google_sheets_list_spreadsheets {} -> allowed"]),
            step("read", "nothing to do this step", &[]),
            step(
                "act",
                "skipped: write state reached before the bound resource was read; no write performed",
                &[],
            ),
            step("answer", "Ta feuille contient 3 prospects.", &[]),
        ],
    };
    let out = format_outcome(&outcome);
    assert!(out.starts_with("Ta feuille contient 3 prospects."));
    assert!(
        !out.contains("skipped:"),
        "the skip is trace-only, not the answer"
    );
}

#[test]
fn format_outcome_without_tools_is_just_the_answer() {
    let outcome = ChainOutcome {
        final_text: "Rien à faire.".into(),
        trace: vec![step("answer", "Rien à faire.", &[])],
    };
    assert_eq!(format_outcome(&outcome), "Rien à faire.");
}

#[test]
fn repair_sheet_links_fixes_garbled_id_when_one_sheet_armed() {
    use flowflow::application::connector_module::repair_sheet_links;
    let armed = vec!["1RealIdAbc_def-123".to_string()];
    let text = "Voici [clients à appeler](https://docs.google.com/spreadsheets/d/1Garbled99/edit).";
    let out = repair_sheet_links(text, &armed);
    assert!(out.contains(
        "https://docs.google.com/spreadsheets/d/1RealIdAbc_def-123/edit"
    ));
    assert!(!out.contains("1Garbled99"));
}

#[test]
fn repair_sheet_links_keeps_a_valid_armed_id() {
    use flowflow::application::connector_module::repair_sheet_links;
    let armed = vec!["1RealIdAbc".to_string(), "1OtherId".to_string()];
    let text = "Ouvre [la feuille](https://docs.google.com/spreadsheets/d/1RealIdAbc/edit#gid=0).";
    assert_eq!(repair_sheet_links(text, &armed), text);
}

#[test]
fn repair_sheet_links_drops_link_on_ambiguous_garble() {
    use flowflow::application::connector_module::repair_sheet_links;
    let armed = vec!["1RealIdAbc".to_string(), "1OtherId".to_string()];
    let text = "Vois [client à appeler](https://docs.google.com/spreadsheets/d/1Wrong/edit) vite.";
    let out = repair_sheet_links(text, &armed);
    assert_eq!(out, "Vois client à appeler vite.");
}

#[test]
fn repair_sheet_links_leaves_non_sheet_text_alone() {
    use flowflow::application::connector_module::repair_sheet_links;
    let armed = vec!["1RealIdAbc".to_string()];
    let text =
        "Pas de lien ici, juste du texte et [une note](flowflow://note/42).";
    assert_eq!(repair_sheet_links(text, &armed), text);
}

// --- armed sheet schema (header capture at arm time) ---

#[test]
fn validate_headers_accepts_unique_names_and_trims() {
    let raw =
        vec!["Date".to_string(), " URL ".to_string(), "Titre".to_string()];
    assert_eq!(
        validate_headers(&raw).unwrap(),
        vec!["Date".to_string(), "URL".to_string(), "Titre".to_string()]
    );
}

#[test]
fn validate_headers_tolerates_an_empty_row() {
    assert_eq!(validate_headers(&[]).unwrap(), Vec::<String>::new());
}

#[test]
fn validate_headers_refuses_an_empty_cell() {
    let raw = vec!["Date".to_string(), "".to_string(), "Titre".to_string()];
    let err = validate_headers(&raw).unwrap_err();
    assert!(err.contains("empty"), "unexpected message: {err}");
}

#[test]
fn validate_headers_refuses_duplicates() {
    let raw = vec!["Date".to_string(), "Date".to_string()];
    let err = validate_headers(&raw).unwrap_err();
    assert!(err.contains("Date"), "unexpected message: {err}");
}

// The real klavis list_sheets shape: tab titles live in sheets[].title.
#[test]
fn sheet_titles_parsed_from_real_meta_shape() {
    let wire = r#"{'spreadsheetId': '1tJDSM5ki0', 'title': 'CRM', 'sheets': [{'sheetId': 0, 'title': 'Feuille 1', 'index': 0}, {'sheetId': 1, 'title': 'Archive', 'index': 1}]}"#;
    let v: serde_json::Value =
        serde_json::from_str(&python_literal_to_json(wire)).unwrap();
    assert_eq!(
        sheet_titles_from_meta(&v),
        vec!["Feuille 1".to_string(), "Archive".to_string()]
    );
}

#[test]
fn sheet_titles_empty_when_meta_has_no_sheets() {
    assert!(sheet_titles_from_meta(&json!({"title": "x"})).is_empty());
}

#[tokio::test]
async fn schema_absent_until_captured_and_cleared_on_unbind() {
    let dir = tempdir().unwrap();
    let db = Database::open_at(dir.path().join("t.db")).unwrap();
    install_arm_agent(&db);

    // Arm without a reachable backend: no schema captured, binding still lands.
    bind_spreadsheet(&db, "S1", "Sheet One").await.unwrap();
    assert_eq!(armed_schema_for(&db, "S1", "Feuille 1"), None);

    // A captured schema round-trips through the settings key and is readable per tab.
    db.set_setting(
        "armed_sheet_schema",
        r#"{"S1": {"Feuille 1": {"headers": ["Date", "URL"], "captured_at": "t"}}}"#,
    )
    .unwrap();
    assert_eq!(
        armed_schema_for(&db, "S1", "Feuille 1"),
        Some(vec!["Date".to_string(), "URL".to_string()])
    );
    assert_eq!(armed_schema_for(&db, "S1", "Autre"), None);

    // Disarming drops the schema with the binding.
    unbind_spreadsheet(&db, "S1").unwrap();
    assert_eq!(armed_schema_for(&db, "S1", "Feuille 1"), None);
}

#[tokio::test]
async fn set_sheet_pins_replaces_only_that_spreadsheet() {
    use flowflow::application::connector_module::{
        binding_entries, set_sheet_pins,
    };
    let dir = tempdir().unwrap();
    let db = Database::open_at(dir.path().join("t.db")).unwrap();
    install_arm_agent(&db);
    bind_spreadsheet(&db, "A", "Alpha").await.unwrap();
    bind_spreadsheet(&db, "B", "Beta").await.unwrap();

    set_sheet_pins(&db, "A", &["X".into(), "Y".into()]).unwrap();
    let entries = binding_entries(&db);
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.spreadsheet_id == "A")
            .filter_map(|e| e.sheet.clone())
            .collect::<Vec<_>>(),
        vec!["X".to_string(), "Y".to_string()]
    );
    assert!(entries
        .iter()
        .any(|e| e.spreadsheet_id == "B" && e.sheet.is_none()));

    // Empty = back to the whole workbook; with no pin left the wire returns to v1.
    set_sheet_pins(&db, "A", &[]).unwrap();
    assert!(binding_entries(&db).iter().all(|e| e.sheet.is_none()));
    let stored = db.get_agent_binding(FIXTURE_AGENT_ID).unwrap();
    let mut ids: Vec<String> = stored["spreadsheet_id"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["A".to_string(), "B".to_string()]);
}
