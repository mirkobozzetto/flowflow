// The manifest -> agent builder: it surfaces the model + governed tools + the real bound_resource +
// the chain, and fails closed on an unarmed connector or governance that does not validate.

use flowflow::application::agent_builder::{
    build_agent, capability_summary, merge_bound, required_connector_types,
};
use flowflow::domain::agent_manifest::parse_manifest;
use flowflow::domain::governance::ConnectorManifest;
use flowflow::infrastructure::persistence::installed_connector_repo::SHEETS_BACKFILL_MANIFEST_JSON;
use serde_json::json;

const VALID_MANIFEST: &str = r#"{
  "schema_version": "1",
  "id": "crm-sync",
  "version": "1.0.0",
  "name": "CRM Sync",
  "model": "gpt-5.4-mini",
  "temperature": 0.1,
  "required_connectors": [
    { "type": "tabular_store", "capabilities": ["search", "read", "update"] }
  ],
  "system_prompt": "Keep the sheet tidy.",
  "governance": {
    "tools": [
      { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" },
      { "tool": "google_sheets_get_spreadsheet",   "mode": "read_only" },
      { "tool": "google_sheets_write_to_cell",     "mode": "read_write" }
    ],
    "bound_resource": { "spreadsheet_id": "sheet-123" },
    "read_before_write": true,
    "deny_destructive": true
  },
  "orchestration": {
    "chains": {
      "sync": {
        "initial": "find",
        "states": {
          "find":   { "allowed_tools": ["google_sheets_list_spreadsheets"], "on_done": "read" },
          "read":   { "allowed_tools": ["google_sheets_get_spreadsheet"], "on_done": "answer" },
          "answer": { "terminal": true }
        }
      }
    }
  }
}"#;

#[test]
fn builds_expected_shape_and_real_bound() {
    let manifest = parse_manifest(VALID_MANIFEST).unwrap();
    let built = build_agent(&manifest, "google", SHEETS_BACKFILL_MANIFEST_JSON)
        .expect("valid manifest builds");

    assert_eq!(built.agent_id, "crm-sync");
    assert_eq!(built.slug(), "google");
    assert_eq!(built.model, "gpt-5.4-mini");
    assert_eq!(built.temperature, Some(0.1));
    // The bound comes from the manifest, not a code constant.
    assert_eq!(
        built.governance.bound_resource,
        Some(json!({ "spreadsheet_id": "sheet-123" }))
    );
    assert!(built.chains.contains_key("sync"));
    built.chains["sync"].validate().expect("chain is sound");
}

#[test]
fn preamble_carries_system_prompt_and_capability_summary() {
    let manifest = parse_manifest(VALID_MANIFEST).unwrap();
    let built = build_agent(&manifest, "google", SHEETS_BACKFILL_MANIFEST_JSON)
        .unwrap();
    assert!(built.preamble.contains("Keep the sheet tidy."));
    assert!(built.preamble.contains("google_sheets_list_spreadsheets"));
    assert!(built.preamble.contains("sheet-123"));
    assert!(built
        .preamble
        .contains("Read the bound resource before any write."));
}

#[test]
fn mission_is_the_manifest_instruction_without_the_capability_summary() {
    let manifest = parse_manifest(VALID_MANIFEST).unwrap();
    let built = build_agent(&manifest, "google", SHEETS_BACKFILL_MANIFEST_JSON)
        .unwrap();
    // The chain runtime prompts with `mission`, one state at a time. A tool list in there would
    // contradict the state's own restriction, so the capability summary must stay out of it.
    assert_eq!(built.mission, "Keep the sheet tidy.");
    assert!(!built.mission.contains("google_sheets_list_spreadsheets"));
    assert!(!built.mission.contains("sheet-123"));
}

#[test]
fn capability_summary_lists_only_governed_tools() {
    let manifest = parse_manifest(VALID_MANIFEST).unwrap();
    let built = build_agent(&manifest, "google", SHEETS_BACKFILL_MANIFEST_JSON)
        .unwrap();
    let summary = capability_summary(&built.governance, &built.connector());
    assert!(summary.contains("google_sheets_get_spreadsheet"));
    // A connector tool the governance does not grant is absent from the summary.
    assert!(!summary.contains("google_sheets_create_spreadsheet"));
}

#[test]
fn required_connector_type_surfaces_the_type_to_resolve() {
    // Resolution is data-driven now (connector_pins): the builder only SURFACES the required
    // type; an unpinned type fails closed at load_built with "no connector pinned".
    let manifest = parse_manifest(VALID_MANIFEST).unwrap();
    assert_eq!(
        required_connector_types(&manifest).unwrap(),
        vec!["tabular_store"]
    );

    let m = VALID_MANIFEST.replace(
        r#""required_connectors": [
    { "type": "tabular_store", "capabilities": ["search", "read", "update"] }
  ],"#,
        r#""required_connectors": [],"#,
    );
    let manifest = parse_manifest(&m).unwrap();
    assert!(required_connector_types(&manifest).is_err());
}

#[test]
fn governance_referencing_unknown_tool_is_rejected() {
    let m = VALID_MANIFEST
        .replace("google_sheets_list_spreadsheets", "google_sheets_nope");
    let manifest = parse_manifest(&m).unwrap();
    let err = build_agent(&manifest, "google", SHEETS_BACKFILL_MANIFEST_JSON)
        .expect_err("unknown tool must fail");
    assert!(err.contains("governance invalid"), "got: {err}");
}

#[test]
fn merge_bound_overlays_install_binding_over_manifest() {
    // Arm-time binding wins key by key; a field only the manifest pins (tab) survives.
    let mut target =
        Some(json!({ "spreadsheet_id": "bound-at-install", "tab": "Clients" }));
    merge_bound(&mut target, json!({ "spreadsheet_id": "1AbC" }));
    assert_eq!(
        target,
        Some(json!({ "spreadsheet_id": "1AbC", "tab": "Clients" }))
    );
}

#[test]
fn merge_bound_sets_when_unbound() {
    let mut target = None;
    merge_bound(&mut target, json!({ "spreadsheet_id": "1AbC" }));
    assert_eq!(target, Some(json!({ "spreadsheet_id": "1AbC" })));
}

#[test]
fn merge_bound_replaces_non_object_wholesale() {
    let mut target = Some(json!("placeholder"));
    merge_bound(&mut target, json!({ "spreadsheet_id": "1AbC" }));
    assert_eq!(target, Some(json!({ "spreadsheet_id": "1AbC" })));
}

#[test]
fn connector_manifest_resolves_for_tabular_store() {
    // The embedded catalog is reachable through a successful build.
    let manifest = parse_manifest(VALID_MANIFEST).unwrap();
    let built = build_agent(&manifest, "google", SHEETS_BACKFILL_MANIFEST_JSON)
        .unwrap();
    let conn: &ConnectorManifest = &built.connector();
    assert_eq!(conn.connector, "google-sheets");
}
