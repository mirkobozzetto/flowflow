// The shared header-keyed row validation (device hook + backend proxy run the same block; the
// conformance corpus pins it). Covers column extraction, schema membership, key policy, and the
// intra-batch duplicate rule.

use flowflow::domain::governance::{
    parse_governance, row_tool_touched_columns, validate_row_batch, DenyReason,
    Governance,
};
use serde_json::{json, Value};

fn gov() -> Governance {
    parse_governance(
        r#"{
          "tools": [
            { "tool": "google_sheets_append_rows", "mode": "append_only" },
            { "tool": "google_sheets_upsert_rows", "mode": "upsert", "key_columns": ["URL"] }
          ],
          "read_before_write": false,
          "deny_destructive": true
        }"#,
    )
    .unwrap()
}

fn append_args(rows: Value) -> Value {
    json!({ "spreadsheet_id": "S", "sheet": "Feuille 1", "rows": rows })
}

fn upsert_args(key_columns: Value, rows: Value) -> Value {
    json!({ "spreadsheet_id": "S", "sheet": "Feuille 1", "key_columns": key_columns, "rows": rows })
}

fn headers(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

#[test]
fn touched_columns_is_the_sorted_deduped_union_of_row_keys() {
    let args = append_args(json!([
        { "Titre": "a", "Date": "d" },
        { "URL": "u", "Date": "d2" }
    ]));
    assert_eq!(
        row_tool_touched_columns(&args),
        vec!["Date".to_string(), "Titre".to_string(), "URL".to_string()]
    );
}

#[test]
fn touched_columns_empty_for_malformed_rows() {
    assert!(
        row_tool_touched_columns(&json!({ "rows": "not an array" })).is_empty()
    );
    assert!(row_tool_touched_columns(&json!({})).is_empty());
}

#[test]
fn non_row_tool_is_ignored() {
    let r = validate_row_batch(
        &gov(),
        "google_sheets_write_to_cell",
        &json!({ "cell": "A1" }),
        Some(&headers(&["Date"])),
    );
    assert!(r.is_none());
}

#[test]
fn append_with_known_columns_passes() {
    let args = append_args(json!([{ "Date": "d", "URL": "u" }]));
    let h = headers(&["Date", "URL", "Titre"]);
    assert!(validate_row_batch(
        &gov(),
        "google_sheets_append_rows",
        &args,
        Some(&h)
    )
    .is_none());
}

#[test]
fn append_with_unknown_column_is_schema_mismatch_naming_the_real_headers() {
    let args = append_args(json!([{ "Date": "d", "Bogus": "x" }]));
    let h = headers(&["Date", "URL"]);
    let r = validate_row_batch(
        &gov(),
        "google_sheets_append_rows",
        &args,
        Some(&h),
    );
    match r {
        Some(DenyReason::SchemaMismatch {
            column, headers, ..
        }) => {
            assert_eq!(column, "Bogus");
            assert_eq!(headers, vec!["Date".to_string(), "URL".to_string()]);
        }
        other => panic!("expected SchemaMismatch, got {other:?}"),
    }
}

#[test]
fn append_on_a_headerless_tab_is_refused() {
    let args = append_args(json!([{ "Date": "d" }]));
    let r = validate_row_batch(
        &gov(),
        "google_sheets_append_rows",
        &args,
        Some(&[]),
    );
    assert!(matches!(r, Some(DenyReason::NoHeaderRow { .. })));
}

#[test]
fn append_without_schema_runs_no_header_check() {
    let args = append_args(json!([{ "Anything": "d" }]));
    assert!(validate_row_batch(
        &gov(),
        "google_sheets_append_rows",
        &args,
        None
    )
    .is_none());
}

#[test]
fn upsert_missing_or_empty_key_columns_is_refused() {
    let no_keys = json!({ "spreadsheet_id": "S", "sheet": "F", "rows": [{ "URL": "u" }] });
    let r =
        validate_row_batch(&gov(), "google_sheets_upsert_rows", &no_keys, None);
    assert!(matches!(r, Some(DenyReason::KeyColumnsMismatch { .. })));

    let empty = upsert_args(json!([]), json!([{ "URL": "u" }]));
    let r =
        validate_row_batch(&gov(), "google_sheets_upsert_rows", &empty, None);
    assert!(matches!(r, Some(DenyReason::KeyColumnsMismatch { .. })));
}

#[test]
fn upsert_with_unauthorized_key_columns_is_refused() {
    let args = upsert_args(json!(["Email"]), json!([{ "Email": "e" }]));
    let r =
        validate_row_batch(&gov(), "google_sheets_upsert_rows", &args, None);
    assert!(matches!(r, Some(DenyReason::KeyColumnsMismatch { .. })));
}

#[test]
fn upsert_key_column_absent_from_headers_is_schema_mismatch() {
    let args = upsert_args(json!(["URL"]), json!([{ "URL": "u" }]));
    let h = headers(&["Date", "Titre"]);
    let r = validate_row_batch(
        &gov(),
        "google_sheets_upsert_rows",
        &args,
        Some(&h),
    );
    assert!(matches!(r, Some(DenyReason::SchemaMismatch { .. })));
}

#[test]
fn upsert_intra_batch_duplicate_key_is_refused_even_without_schema() {
    let args = upsert_args(
        json!(["URL"]),
        json!([
            { "URL": "https://a", "Titre": "one" },
            { "URL": "https://a", "Titre": "two" }
        ]),
    );
    let r =
        validate_row_batch(&gov(), "google_sheets_upsert_rows", &args, None);
    match r {
        Some(DenyReason::DuplicateKeyInBatch { key, .. }) => {
            assert_eq!(key, "https://a");
        }
        other => panic!("expected DuplicateKeyInBatch, got {other:?}"),
    }
}

#[test]
fn upsert_happy_path_passes_with_schema() {
    let args = upsert_args(
        json!(["URL"]),
        json!([
            { "URL": "https://a", "Titre": "one" },
            { "URL": "https://b", "Titre": "two" }
        ]),
    );
    let h = headers(&["Date", "URL", "Titre"]);
    assert!(validate_row_batch(
        &gov(),
        "google_sheets_upsert_rows",
        &args,
        Some(&h)
    )
    .is_none());
}
