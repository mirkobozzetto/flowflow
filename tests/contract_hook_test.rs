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
    let hook = ContractHook::with_contract(
        tx,
        governed(),
        sheets(),
        Default::default(),
    )
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

// --- header-keyed row writes: schema validation at the hook seam (no peer = no re-sync) ---

fn row_manifest() -> ConnectorManifest {
    parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store",
          "server": "s", "mcp_prefix": "google_sheets_",
          "provides": ["read", "append"],
          "tools": [
            { "tool": "google_sheets_get_spreadsheet", "resource": "spreadsheet", "action": "read",   "risk": "read_only" },
            { "tool": "google_sheets_append_rows",     "resource": "row",         "action": "append", "risk": "read_write" }
          ]
        }"#,
    )
    .unwrap()
}

fn row_governed() -> Governance {
    parse_governance(
        r#"{
          "tools": [
            { "tool": "google_sheets_get_spreadsheet", "mode": "read_only" },
            { "tool": "google_sheets_append_rows",     "mode": "append_only" }
          ],
          "bound_resource": { "spreadsheet_id": "SHEET_A" },
          "read_before_write": false,
          "deny_destructive": true
        }"#,
    )
    .unwrap()
}

fn schema_snapshot(
    headers: &[&str],
) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert(
        "SHEET_A".into(),
        json!({ "Feuille 1": { "headers": headers, "captured_at": "t" } }),
    );
    m
}

fn row_hook(
    schema: serde_json::Map<String, serde_json::Value>,
) -> ContractHook {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    ContractHook::with_contract(tx, row_governed(), row_manifest(), schema)
}

async fn propose_append(
    hook: &ContractHook,
    rows: serde_json::Value,
) -> ToolCallHookAction {
    PromptHook::<Model>::on_tool_call(
        hook,
        "google_sheets_append_rows",
        None,
        "",
        &json!({ "spreadsheet_id": "SHEET_A", "sheet": "Feuille 1", "rows": rows }).to_string(),
    )
    .await
}

#[tokio::test]
async fn append_matching_the_captured_schema_passes() {
    let hook = row_hook(schema_snapshot(&["Date", "URL", "Titre"]));
    let action =
        propose_append(&hook, json!([{ "Date": "d", "URL": "u" }])).await;
    assert!(matches!(action, ToolCallHookAction::Continue));
}

#[tokio::test]
async fn append_with_unknown_column_is_refused_naming_the_real_headers() {
    let hook = row_hook(schema_snapshot(&["Date", "URL"]));
    let action = propose_append(&hook, json!([{ "Bogus": "x" }])).await;
    match action {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("Bogus"), "names the offender: {reason}");
            assert!(
                reason.contains("Date"),
                "names the real headers: {reason}"
            );
        }
        other => panic!("expected Skip, got {other:?}"),
    }
}

#[tokio::test]
async fn append_on_a_headerless_tab_is_refused() {
    let hook = row_hook(schema_snapshot(&[]));
    let action = propose_append(&hook, json!([{ "Date": "d" }])).await;
    match action {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("no header row"), "{reason}");
        }
        other => panic!("expected Skip, got {other:?}"),
    }
}

#[tokio::test]
async fn append_without_captured_schema_and_no_peer_refuses_blind_write() {
    let hook = row_hook(serde_json::Map::new());
    let action = propose_append(&hook, json!([{ "Date": "d" }])).await;
    match action {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("not captured"), "{reason}");
        }
        other => panic!("expected Skip, got {other:?}"),
    }
}

// --- Multi-contract resolution (the chat surface's hook shape) ---

fn exa() -> ConnectorManifest {
    parse_connector_manifest(
        r#"{
          "connector": "exa", "type": "web_search",
          "server": "s", "mcp_prefix": "exa_",
          "provides": ["search"],
          "tools": [
            { "tool": "exa_search", "resource": "web", "action": "search", "risk": "read_only" }
          ]
        }"#,
    )
    .unwrap()
}

fn exa_gov() -> Governance {
    parse_governance(
        r#"{
          "tools": [
            { "tool": "exa_search", "mode": "read_only", "approval": "require_approval" }
          ],
          "read_before_write": false,
          "deny_destructive": true
        }"#,
    )
    .unwrap()
}

fn chat_sheets_gov() -> Governance {
    parse_governance(
        r#"{
          "tools": [
            { "tool": "google_sheets_get_spreadsheet", "mode": "read_only",  "approval": "require_approval" },
            { "tool": "google_sheets_write_to_cell",   "mode": "read_write", "approval": "require_approval" }
          ],
          "read_before_write": false,
          "deny_destructive": true
        }"#,
    )
    .unwrap()
}

fn multi_hook(
    timeout: Duration,
) -> (ContractHook, UnboundedReceiver<ToolEvent>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let hook = ContractHook::with_contracts(
        tx,
        vec![
            ("google_sheets_".to_string(), chat_sheets_gov(), sheets()),
            ("exa_".to_string(), exa_gov(), exa()),
        ],
    )
    .with_approval_timeout(timeout);
    (hook, rx)
}

async fn call(hook: &ContractHook, tool: &str) -> ToolCallHookAction {
    PromptHook::<Model>::on_tool_call(hook, tool, None, "", "{}").await
}

#[tokio::test]
async fn multi_contract_reads_resolve_per_connector_and_pass() {
    let (hook, _rx) = multi_hook(Duration::from_millis(200));
    // Each read resolves to ITS connector's contract: RequireApproval holds only writes.
    assert!(matches!(
        call(&hook, "google_sheets_get_spreadsheet").await,
        ToolCallHookAction::Continue
    ));
    assert!(matches!(
        call(&hook, "exa_search").await,
        ToolCallHookAction::Continue
    ));
}

#[tokio::test]
async fn multi_contract_write_holds_and_reject_aborts_the_run() {
    let (hook, mut rx) = multi_hook(Duration::from_secs(5));
    assert!(!hook.aborted());
    let pending = tokio::spawn({
        let hook = hook.clone();
        async move { call(&hook, "google_sheets_write_to_cell").await }
    });
    let id = next_proposal_id(&mut rx).await;
    decide(id, UserDecision::Rejected).unwrap();
    let action = pending.await.unwrap();
    assert!(matches!(action, ToolCallHookAction::Skip { .. }));
    assert!(
        hook.aborted(),
        "reject flags the run as aborted (any contract)"
    );
}

#[tokio::test]
async fn multi_contract_unmatched_mcp_tool_is_refused() {
    let (hook, _rx) = multi_hook(Duration::from_millis(200));
    match call(&hook, "stripe_create_charge").await {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("not governed"), "{reason}");
        }
        other => panic!("expected Skip, got {other:?}"),
    }
}

#[tokio::test]
async fn multi_contract_notes_tools_pass_untouched() {
    let (hook, _rx) = multi_hook(Duration::from_millis(200));
    for tool in ["search_notes", "create_note", "summarize_folder"] {
        assert!(matches!(
            call(&hook, tool).await,
            ToolCallHookAction::Continue
        ));
    }
}

#[tokio::test]
async fn after_reject_every_later_call_is_refused_without_a_new_card() {
    let (hook, mut rx) = multi_hook(Duration::from_secs(5));
    let pending = tokio::spawn({
        let hook = hook.clone();
        async move { call(&hook, "google_sheets_write_to_cell").await }
    });
    let id = next_proposal_id(&mut rx).await;
    decide(id, UserDecision::Rejected).unwrap();
    let _ = pending.await.unwrap();

    // The model retrying the write - or reaching for ANY tool - gets refused outright:
    // no second proposal event, the run concludes on what it already has.
    match call(&hook, "google_sheets_write_to_cell").await {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("ended this run"), "{reason}");
        }
        other => panic!("expected Skip, got {other:?}"),
    }
    match call(&hook, "google_sheets_get_spreadsheet").await {
        ToolCallHookAction::Skip { .. } => {}
        other => panic!("expected Skip, got {other:?}"),
    }
    let mut proposals = 0;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, ToolEvent::Proposal(_)) {
            proposals += 1;
        }
    }
    // The original card was already consumed by next_proposal_id above.
    assert_eq!(proposals, 0, "no second card after the reject");
}

// ---- RFC 0023: scoped tools, executed-and-filtered by the hook ----

fn sheets_scoped() -> ConnectorManifest {
    parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store",
          "server": "s", "mcp_prefix": "google_sheets_",
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

fn scoped_hook(bound: serde_json::Value) -> ContractHook {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let gov = parse_governance(
        &json!({
          "tools": [
            { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" },
            { "tool": "google_sheets_get_spreadsheet",   "mode": "read_only" }
          ],
          "bound_resource": bound,
          "read_before_write": false,
          "deny_destructive": true
        })
        .to_string(),
    )
    .unwrap();
    ContractHook::with_contract(tx, gov, sheets_scoped(), Default::default())
}

#[tokio::test]
async fn scoped_list_never_runs_raw_and_fails_closed_without_a_peer() {
    // Armed: the gate allows the declared list, the hook must execute-and-filter it. With no
    // peer attached, the result is the EMPTY shape + fixed note - never the raw tool result.
    let hook =
        scoped_hook(json!([{ "spreadsheet_id": "A", "sheet": "Clients" }]));
    let action = PromptHook::<Model>::on_tool_call(
        &hook,
        "google_sheets_list_spreadsheets",
        None,
        "",
        "{}",
    )
    .await;
    match action {
        ToolCallHookAction::Skip { reason } => {
            let v: serde_json::Value = serde_json::from_str(&reason).unwrap();
            assert_eq!(v["spreadsheets"], json!([]));
            assert_eq!(v["note"], "scoped to armed resources");
            // The note never carries a count.
            assert!(!reason.contains("total_count"), "{reason}");
        }
        other => panic!("expected Skip with filtered result, got {other:?}"),
    }
}

#[tokio::test]
async fn tabs_read_is_filtered_only_when_the_matched_entry_pins_a_tab() {
    // Whole-workbook arming: the read passes through untouched (Continue).
    let hook = scoped_hook(json!([{ "spreadsheet_id": "A" }]));
    let action = PromptHook::<Model>::on_tool_call(
        &hook,
        "google_sheets_get_spreadsheet",
        None,
        "",
        &json!({"spreadsheet_id": "A"}).to_string(),
    )
    .await;
    assert!(matches!(action, ToolCallHookAction::Continue));

    // Tab-pinned arming: the hook owns the read; no peer = empty shape + note, never raw.
    let hook =
        scoped_hook(json!([{ "spreadsheet_id": "A", "sheet": "Clients" }]));
    let action = PromptHook::<Model>::on_tool_call(
        &hook,
        "google_sheets_get_spreadsheet",
        None,
        "",
        &json!({"spreadsheet_id": "A"}).to_string(),
    )
    .await;
    match action {
        ToolCallHookAction::Skip { reason } => {
            let v: serde_json::Value = serde_json::from_str(&reason).unwrap();
            assert_eq!(v["sheets"], json!([]));
            assert_eq!(v["note"], "scoped to armed resources");
        }
        other => panic!("expected Skip, got {other:?}"),
    }
}

#[tokio::test]
async fn nothing_armed_keeps_free_discovery() {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let gov = parse_governance(
        r#"{
          "tools": [ { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" } ],
          "read_before_write": false, "deny_destructive": true
        }"#,
    )
    .unwrap();
    let hook = ContractHook::with_contract(
        tx,
        gov,
        sheets_scoped(),
        Default::default(),
    );
    let action = PromptHook::<Model>::on_tool_call(
        &hook,
        "google_sheets_list_spreadsheets",
        None,
        "",
        "{}",
    )
    .await;
    assert!(matches!(action, ToolCallHookAction::Continue));
}

#[tokio::test]
async fn undeclared_search_with_bound_is_denied_not_executed() {
    // A manifest WITHOUT scoping: the gate refuses the search outright; the hook never runs it.
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let conn = parse_connector_manifest(
        r#"{
          "connector": "google-sheets", "type": "tabular_store",
          "server": "s", "mcp_prefix": "google_sheets_",
          "provides": ["search"],
          "tools": [
            { "tool": "google_sheets_list_spreadsheets", "resource": "spreadsheet", "action": "search", "risk": "read_only" }
          ]
        }"#,
    )
    .unwrap();
    let gov = parse_governance(
        r#"{
          "tools": [ { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" } ],
          "bound_resource": [{ "spreadsheet_id": "A" }],
          "read_before_write": false, "deny_destructive": true
        }"#,
    )
    .unwrap();
    let hook = ContractHook::with_contract(tx, gov, conn, Default::default());
    let action = PromptHook::<Model>::on_tool_call(
        &hook,
        "google_sheets_list_spreadsheets",
        None,
        "",
        "{}",
    )
    .await;
    match action {
        ToolCallHookAction::Skip { reason } => {
            assert!(reason.contains("scoped to armed resources"), "{reason}");
            assert!(reason.contains("cannot be filtered"), "{reason}");
        }
        other => panic!("expected Skip deny, got {other:?}"),
    }
}

// ---- the pure filter, pinned against the real wire shapes ----

use flowflow::application::connector_module::python_literal_to_json;
use flowflow::application::tools::{filter_items, path_str};

#[test]
fn filter_keeps_armed_ids_and_rewrites_total_count() {
    // The real klavis list shape travels as a Python repr; the armed filter runs after the
    // same normalization the arm-time parser uses.
    let wire = r#"{'spreadsheets': [{'id': 'A', 'name': 'Clients'}, {'id': 'LEAK', 'name': 'Salaires'}], 'total_count': 2}"#;
    let v: serde_json::Value =
        serde_json::from_str(&python_literal_to_json(wire)).unwrap();
    let filtered = filter_items(v, "spreadsheets", |item| {
        path_str(item, "id").is_some_and(|id| id == "A")
    })
    .unwrap();
    assert_eq!(
        filtered["spreadsheets"],
        json!([{"id": "A", "name": "Clients"}])
    );
    assert_eq!(filtered["total_count"], json!(1));
}

#[test]
fn filter_masks_tabs_by_dotted_name_key() {
    let response = json!({
        "spreadsheetId": "A",
        "sheets": [
            { "properties": { "title": "Clients" }, "data": [1] },
            { "properties": { "title": "Salaires" }, "data": [2] }
        ]
    });
    let filtered = filter_items(response, "sheets", |item| {
        path_str(item, "properties.title").is_some_and(|t| t == "Clients")
    })
    .unwrap();
    assert_eq!(filtered["sheets"].as_array().unwrap().len(), 1);
    assert_eq!(filtered["sheets"][0]["properties"]["title"], "Clients");
}

#[test]
fn filter_fails_closed_on_missing_or_non_array_path() {
    assert!(
        filter_items(json!({"other": []}), "spreadsheets", |_| true).is_none()
    );
    assert!(filter_items(
        json!({"spreadsheets": "oops"}),
        "spreadsheets",
        |_| true
    )
    .is_none());
}
