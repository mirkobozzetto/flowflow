// Drift guard for the DEVICE copy of the governance gate (RFC 0010 M1.12). The schema + gate are agreed
// once with the backend (marketplace-flowflow/src/governance.rs); these cases mirror the backend's gate
// semantics so a divergence in the on-device contract is caught here. The device runs the FULL gate
// (it owns run state), so read_before_write + budgets are exercised, unlike the stateless proxy seam.

use flowflow::domain::governance::{
    gate, parse_connector_manifest, parse_governance, validate_governance,
    ConnectorManifest, Decision, DenyReason, Governance, GovernanceError, Mode,
    ProposedCall, RunState,
};
use serde_json::json;

fn sheets() -> ConnectorManifest {
    parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store",
          "server": "ghcr.io/klavis-ai/google-sheets-mcp-server", "mcp_prefix": "google_sheets_",
          "provides": ["search", "read", "create", "update"],
          "tools": [
            { "tool": "google_sheets_list_spreadsheets", "resource": "spreadsheet", "action": "search", "risk": "read_only" },
            { "tool": "google_sheets_get_spreadsheet",    "resource": "spreadsheet", "action": "read",   "risk": "read_only" },
            { "tool": "google_sheets_write_to_cell",      "resource": "cell",        "action": "update", "risk": "read_write" }
          ]
        }"#,
    )
    .unwrap()
}

fn synthetic_destructive() -> ConnectorManifest {
    parse_connector_manifest(
        r#"{
          "connector": "syn", "type": "tabular_store", "server": "t", "mcp_prefix": "syn_",
          "provides": ["update"],
          "tools": [ { "tool": "syn_wipe", "resource": "row", "action": "update", "risk": "destructive" } ]
        }"#,
    )
    .unwrap()
}

fn governed() -> Governance {
    parse_governance(
        r#"{
          "tools": [
            { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" },
            { "tool": "google_sheets_get_spreadsheet",    "mode": "read_only" },
            { "tool": "google_sheets_write_to_cell",      "mode": "read_write" }
          ],
          "bound_resource": { "spreadsheet_id": "SHEET1" },
          "read_before_write": true,
          "deny_destructive": true
        }"#,
    )
    .unwrap()
}

fn bound_args() -> serde_json::Value {
    json!({ "spreadsheet_id": "SHEET1", "cell": "B2", "value": "x" })
}

#[test]
fn allowlist_default_deny() {
    // create_spreadsheet is not in the governed tools[] (and not in this manifest either).
    let call =
        ProposedCall::new("google_sheets_create_spreadsheet", bound_args());
    let mut run = RunState::default();
    assert_eq!(
        gate(&governed(), &sheets(), &call, &mut run),
        Decision::Deny(DenyReason::NotAllowed {
            tool: "google_sheets_create_spreadsheet".into()
        })
    );
}

#[test]
fn read_only_cannot_write() {
    let gov = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_write_to_cell", "mode": "read_only" } ],
             "bound_resource": { "spreadsheet_id": "SHEET1" } }"#,
    )
    .unwrap();
    // The mode check precedes read_before_write, so a default run still denies on the mode mismatch.
    let mut run = RunState::default();
    assert_eq!(
        gate(
            &gov,
            &sheets(),
            &ProposedCall::new("google_sheets_write_to_cell", bound_args()),
            &mut run
        ),
        Decision::Deny(DenyReason::ModeActionMismatch {
            tool: "google_sheets_write_to_cell".into(),
            mode: Mode::ReadOnly,
            action: flowflow::domain::governance::Action::Update,
        })
    );
}

#[test]
fn off_bound_write_denied() {
    let call = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "OTHER", "cell": "A1" }),
    );
    // check_bound precedes read_before_write, so a default run still denies as off-bound.
    let mut run = RunState::default();
    assert_eq!(
        gate(&governed(), &sheets(), &call, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource {
            tool: "google_sheets_write_to_cell".into()
        })
    );
}

#[test]
fn write_before_read_denied_then_allowed_after_read() {
    let gov = governed();
    let conn = sheets();
    let write = ProposedCall::new("google_sheets_write_to_cell", bound_args());
    let mut run = RunState::default();
    assert_eq!(
        gate(&gov, &conn, &write, &mut run),
        Decision::Deny(DenyReason::ReadBeforeWrite {
            tool: "google_sheets_write_to_cell".into()
        })
    );
    // a read of the bound resource satisfies read_before_write for the run.
    let read = ProposedCall::new("google_sheets_get_spreadsheet", bound_args());
    assert!(gate(&gov, &conn, &read, &mut run).is_allowed());
    assert!(!run.read_resources.is_empty());
    assert!(gate(&gov, &conn, &write, &mut run).is_allowed());
}

#[test]
fn destructive_floor_denies() {
    let gov = parse_governance(
        r#"{ "tools": [ { "tool": "syn_wipe", "mode": "read_write" } ], "deny_destructive": true,
             "read_before_write": false }"#,
    )
    .unwrap();
    let mut run = RunState::default();
    assert_eq!(
        gate(
            &gov,
            &synthetic_destructive(),
            &ProposedCall::new("syn_wipe", json!({})),
            &mut run
        ),
        Decision::Deny(DenyReason::Destructive {
            tool: "syn_wipe".into()
        })
    );
}

#[test]
fn write_grant_under_read_before_write_needs_a_bound_resource() {
    // A real write grant with read_before_write on but no pinned target is rejected at validation: the
    // global read flag would otherwise mean only "some read happened", not "read what you write".
    let unbound = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_write_to_cell", "mode": "read_write" } ],
             "read_before_write": true }"#,
    )
    .unwrap();
    let errors = validate_governance(&unbound, &sheets()).unwrap_err();
    assert!(
        errors.contains(&GovernanceError::ReadBeforeWriteWithoutBound {
            tool: "google_sheets_write_to_cell".into()
        })
    );

    // Pinning the target clears it; a read_only grant never trips the rule either.
    let bound = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_write_to_cell", "mode": "read_write" } ],
             "bound_resource": { "spreadsheet_id": "SHEET1" }, "read_before_write": true }"#,
    )
    .unwrap();
    validate_governance(&bound, &sheets())
        .expect("pinned write grant validates");
}

#[test]
fn deny_reason_serializes_structured() {
    // the Skip{reason} string is what the model sees; the structured form is what a log keeps.
    let r = DenyReason::ReadBeforeWrite {
        tool: "google_sheets_write_to_cell".into(),
    };
    assert_eq!(
        r.to_string(),
        "tool `google_sheets_write_to_cell`: write attempted before reading the bound resource this run"
    );
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["deny"], "read_before_write");
}

// ---- RFC 0011: multi-resource binding ----

fn multi() -> Governance {
    parse_governance(
        r#"{ "tools": [
               { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" },
               { "tool": "google_sheets_write_to_cell",   "mode": "read_write" } ],
             "bound_resource": { "spreadsheet_id": ["A", "B"] },
             "read_before_write": true }"#,
    )
    .unwrap()
}

#[test]
fn array_bound_allows_member_and_denies_non_member() {
    let (gov, conn) = (multi(), sheets());
    let mut run = RunState::default();
    let read_b = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "B" }),
    );
    assert!(gate(&gov, &conn, &read_b, &mut run).is_allowed());
    let write_b = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "B", "cell": "A1", "value": "x" }),
    );
    assert!(gate(&gov, &conn, &write_b, &mut run).is_allowed());

    let read_c = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "C" }),
    );
    assert_eq!(
        gate(&gov, &conn, &read_c, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource {
            tool: "google_sheets_get_spreadsheet".into()
        })
    );
}

#[test]
fn per_resource_read_does_not_satisfy_a_sibling() {
    let (gov, conn) = (multi(), sheets());
    let mut run = RunState::default();
    let read_a = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "A" }),
    );
    assert!(gate(&gov, &conn, &read_a, &mut run).is_allowed());
    // reading A does not unlock a write to its sibling B.
    let write_b = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "B", "cell": "A1", "value": "x" }),
    );
    assert_eq!(
        gate(&gov, &conn, &write_b, &mut run),
        Decision::Deny(DenyReason::ReadBeforeWrite {
            tool: "google_sheets_write_to_cell".into()
        })
    );
    // reading B unlocks the write to B.
    let read_b = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "B" }),
    );
    assert!(gate(&gov, &conn, &read_b, &mut run).is_allowed());
    assert!(gate(&gov, &conn, &write_b, &mut run).is_allowed());
}

#[test]
fn resource_key_is_independent_of_arg_key_order() {
    let gov = parse_governance(
        r#"{ "tools": [
               { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" },
               { "tool": "google_sheets_write_to_cell",   "mode": "read_write" } ],
             "bound_resource": { "spreadsheet_id": "A", "tab": "t1" },
             "read_before_write": true }"#,
    )
    .unwrap();
    let conn = sheets();
    let mut run = RunState::default();
    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "A", "tab": "t1" }),
    );
    assert!(gate(&gov, &conn, &read, &mut run).is_allowed());
    // the write spells the bound fields in a different order: same resource, still satisfied.
    let write = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "tab": "t1", "value": "x", "spreadsheet_id": "A" }),
    );
    assert!(gate(&gov, &conn, &write, &mut run).is_allowed());
}

#[test]
fn validate_rejects_malformed_bound() {
    let empty = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" } ],
             "bound_resource": { "spreadsheet_id": [] } }"#,
    )
    .unwrap();
    assert!(validate_governance(&empty, &sheets())
        .unwrap_err()
        .iter()
        .any(|e| matches!(e, GovernanceError::MalformedBoundResource { .. })));

    let mixed = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" } ],
             "bound_resource": { "spreadsheet_id": ["A", 1] } }"#,
    )
    .unwrap();
    assert!(validate_governance(&mixed, &sheets())
        .unwrap_err()
        .iter()
        .any(|e| matches!(e, GovernanceError::MalformedBoundResource { .. })));
}

// ---- binding v2: OR-of-AND entries with tab pinning (RFC 0023) ----

fn sheets_with_scoping() -> ConnectorManifest {
    parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store", "server": "t", "mcp_prefix": "google_sheets_",
          "provides": ["search", "read", "update"],
          "tools": [
            { "tool": "google_sheets_list_spreadsheets", "resource": "spreadsheet", "action": "search", "risk": "read_only" },
            { "tool": "google_sheets_get_spreadsheet",   "resource": "spreadsheet", "action": "read",   "risk": "read_only" },
            { "tool": "google_sheets_write_to_cell",     "resource": "cell",        "action": "update", "risk": "read_write" }
          ],
          "scoping": {
            "id_field": "spreadsheet_id",
            "list": { "tool": "google_sheets_list_spreadsheets", "items_path": "spreadsheets", "id_key": "id" },
            "tabs": { "tool": "google_sheets_get_spreadsheet", "items_path": "sheets", "name_key": "properties.title" }
          }
        }"#,
    )
    .unwrap()
}

fn v2_bound() -> Governance {
    parse_governance(
        r#"{ "tools": [
               { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" },
               { "tool": "google_sheets_write_to_cell",   "mode": "read_write" } ],
             "bound_resource": [
               { "spreadsheet_id": "A", "sheet": "Clients" },
               { "spreadsheet_id": "B" } ],
             "read_before_write": true }"#,
    )
    .unwrap()
}

#[test]
fn v2_bound_validates() {
    validate_governance(&v2_bound(), &sheets()).expect("v2 array validates");
}

#[test]
fn v2_read_gates_on_id_only_and_write_needs_the_pinned_tab() {
    let (gov, conn) = (v2_bound(), sheets());
    let mut run = RunState::default();

    // A read carries only the id: the tab pin is filtered in the response, not arg-gated.
    let read_a = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "A" }),
    );
    assert!(gate(&gov, &conn, &read_a, &mut run).is_allowed());
    // Off-bound id still denied.
    let read_c = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "C" }),
    );
    assert!(matches!(
        gate(&gov, &conn, &read_c, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource { .. })
    ));

    // A write on the armed tab passes; the sibling tab is refused.
    let write_ok = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "A", "sheet": "Clients", "cell": "B2", "value": "x" }),
    );
    assert!(gate(&gov, &conn, &write_ok, &mut run).is_allowed());
    let write_other_tab = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "A", "sheet": "Salaires", "cell": "B2", "value": "x" }),
    );
    assert!(matches!(
        gate(&gov, &conn, &write_other_tab, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource { .. })
    ));
    // A write that addresses no tab at all under a tab pin is refused (fail closed).
    let write_no_tab = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "A", "cell": "B2", "value": "x" }),
    );
    assert!(matches!(
        gate(&gov, &conn, &write_no_tab, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource { .. })
    ));
}

#[test]
fn v2_write_addresses_the_tab_via_a1_prefix() {
    let (gov, conn) = (v2_bound(), sheets());
    let mut run = RunState::default();
    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "A" }),
    );
    assert!(gate(&gov, &conn, &read, &mut run).is_allowed());

    let bare = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "A", "cell": "Clients!B2", "value": "x" }),
    );
    assert!(gate(&gov, &conn, &bare, &mut run).is_allowed());
    let quoted = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "A", "cell": "'Clients'!B2", "value": "x" }),
    );
    assert!(gate(&gov, &conn, &quoted, &mut run).is_allowed());
    let off_pin = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "A", "cell": "Salaires!B2", "value": "x" }),
    );
    assert!(matches!(
        gate(&gov, &conn, &off_pin, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource { .. })
    ));
}

#[test]
fn v2_reading_armed_tab_x_does_not_unlock_write_to_armed_tab_y() {
    // Adversarial: two entries pin two tabs of the same spreadsheet. A read that addresses
    // tab X matches only X's entry, so a write to Y still needs its own read.
    let gov = parse_governance(
        r#"{ "tools": [
               { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" },
               { "tool": "google_sheets_write_to_cell",   "mode": "read_write" } ],
             "bound_resource": [
               { "spreadsheet_id": "A", "sheet": "X" },
               { "spreadsheet_id": "A", "sheet": "Y" } ],
             "read_before_write": true }"#,
    )
    .unwrap();
    let conn = sheets();
    let mut run = RunState::default();

    let read_x = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "A", "sheet": "X" }),
    );
    assert!(gate(&gov, &conn, &read_x, &mut run).is_allowed());
    let write_y = ProposedCall::new(
        "google_sheets_write_to_cell",
        json!({ "spreadsheet_id": "A", "sheet": "Y", "cell": "B2", "value": "x" }),
    );
    assert!(matches!(
        gate(&gov, &conn, &write_y, &mut run),
        Decision::Deny(DenyReason::ReadBeforeWrite { .. })
    ));

    // A whole-spreadsheet read covers every armed tab it matches, unlocking Y.
    let read_all = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "A" }),
    );
    assert!(gate(&gov, &conn, &read_all, &mut run).is_allowed());
    assert!(gate(&gov, &conn, &write_y, &mut run).is_allowed());
}

#[test]
fn search_with_pinned_bound_is_denied_unless_scoping_declares_it() {
    let gov = v2_bound();
    let list = ProposedCall::new("google_sheets_list_spreadsheets", json!({}));

    // The governed tools[] must include the list for it to resolve at all.
    let gov = {
        let mut g = gov;
        g.tools.push(
            parse_governance(
                r#"{ "tools": [ { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" } ] }"#,
            )
            .unwrap()
            .tools
            .remove(0),
        );
        g
    };

    // No scoping in the manifest: the response cannot be filtered, fail closed.
    let mut run = RunState::default();
    assert!(matches!(
        gate(&gov, &sheets(), &list, &mut run),
        Decision::Deny(DenyReason::UnscopedSearch { .. })
    ));

    // Declared in scoping: the gate exempts it (the hook executes and filters it).
    let mut run = RunState::default();
    assert!(gate(&gov, &sheets_with_scoping(), &list, &mut run).is_allowed());

    // Nothing pinned: free discovery, with or without scoping.
    let mut unpinned = gov.clone();
    unpinned.bound_resource = None;
    let mut run = RunState::default();
    assert!(gate(&unpinned, &sheets(), &list, &mut run).is_allowed());
}

#[test]
fn unparseable_bound_fails_closed() {
    // A bound that is neither object nor array pins everything shut: reads deny out-of-bound,
    // Search denies unscoped. A corrupt binding must never widen the scope.
    let gov = parse_governance(
        r#"{ "tools": [
               { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" },
               { "tool": "google_sheets_get_spreadsheet",   "mode": "read_only" } ],
             "bound_resource": "GARBAGE", "read_before_write": false }"#,
    )
    .unwrap();
    let conn = sheets();
    let mut run = RunState::default();
    let read = ProposedCall::new(
        "google_sheets_get_spreadsheet",
        json!({ "spreadsheet_id": "A" }),
    );
    assert!(matches!(
        gate(&gov, &conn, &read, &mut run),
        Decision::Deny(DenyReason::OutOfBoundResource { .. })
    ));
    let list = ProposedCall::new("google_sheets_list_spreadsheets", json!({}));
    assert!(matches!(
        gate(&gov, &conn, &list, &mut run),
        Decision::Deny(DenyReason::UnscopedSearch { .. })
    ));
}

#[test]
fn v2_validate_rejects_empty_and_malformed_entries() {
    let empty_array = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" } ],
             "bound_resource": [] }"#,
    )
    .unwrap();
    assert!(validate_governance(&empty_array, &sheets())
        .unwrap_err()
        .iter()
        .any(|e| matches!(e, GovernanceError::MalformedBoundResource { .. })));

    let scalar_entry = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" } ],
             "bound_resource": [ "A" ] }"#,
    )
    .unwrap();
    assert!(validate_governance(&scalar_entry, &sheets())
        .unwrap_err()
        .iter()
        .any(|e| matches!(e, GovernanceError::MalformedBoundResource { .. })));

    let empty_entry = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" } ],
             "bound_resource": [ {} ] }"#,
    )
    .unwrap();
    assert!(validate_governance(&empty_entry, &sheets())
        .unwrap_err()
        .iter()
        .any(|e| matches!(e, GovernanceError::MalformedBoundResource { .. })));
}

// ---- manifest `scoping` block (RFC 0023) ----

#[test]
fn scoping_block_parses_and_declares_tools() {
    let conn = parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store", "server": "t", "mcp_prefix": "google_sheets_",
          "provides": ["search", "read"],
          "tools": [
            { "tool": "google_sheets_list_spreadsheets", "resource": "spreadsheet", "action": "search", "risk": "read_only" },
            { "tool": "google_sheets_get_spreadsheet",   "resource": "spreadsheet", "action": "read",   "risk": "read_only" }
          ],
          "scoping": {
            "id_field": "spreadsheet_id",
            "list": { "tool": "google_sheets_list_spreadsheets", "items_path": "spreadsheets", "id_key": "id" },
            "tabs": { "tool": "google_sheets_get_spreadsheet", "items_path": "sheets", "name_key": "properties.title" }
          }
        }"#,
    )
    .unwrap();
    let scoping = conn.scoping.expect("scoping parsed");
    assert_eq!(scoping.id_field, "spreadsheet_id");
    assert!(scoping.declares("google_sheets_list_spreadsheets"));
    assert!(scoping.declares("google_sheets_get_spreadsheet"));
    assert!(!scoping.declares("google_sheets_write_to_cell"));
}

#[test]
fn manifest_without_scoping_yields_none() {
    assert!(sheets().scoping.is_none());
}

#[test]
fn malformed_scoping_fails_the_manifest_parse() {
    // Fail closed: a scoping block missing required keys drops the whole manifest.
    let res = parse_connector_manifest(
        r#"{
          "connector": "syn", "type": "tabular_store", "server": "t", "mcp_prefix": "syn_",
          "provides": ["search"],
          "tools": [ { "tool": "syn_list", "resource": "x", "action": "search", "risk": "read_only" } ],
          "scoping": { "list": { "tool": "syn_list" } }
        }"#,
    );
    assert!(res.is_err());
}

#[test]
fn scalar_bound_under_write_grant_is_rejected() {
    // A non-object, non-array bound is structurally malformed: refused at install. At runtime
    // it would pin everything shut (fail closed), never silently widen.
    let scalar = parse_governance(
        r#"{ "tools": [ { "tool": "google_sheets_write_to_cell", "mode": "read_write" } ],
             "bound_resource": "SHEET1", "read_before_write": true }"#,
    )
    .unwrap();
    let errors = validate_governance(&scalar, &sheets()).unwrap_err();
    assert!(errors
        .iter()
        .any(|e| matches!(e, GovernanceError::MalformedBoundResource { .. })));
}
