// Agent directory: the backend /v1/agents payload parses into summaries (extra fields
// ignored), the install-state join is correct, and install/uninstall behave offline
// (no backend -> clear error; uninstall drops only the pinned row).

use flowflow::application::agent_directory::{
    ensure_installed, list_agents, mark_installed, uninstall, AgentEntry,
};
use flowflow::application::connector_module::FIXTURE_PACKAGE;
use flowflow::domain::agent_manifest::{verify_package, ADMIN_PUBKEY};
use flowflow::infrastructure::backend::AgentSummary;
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_db(dir: &tempfile::TempDir) -> Database {
    Database::open_at(dir.path().join("t.db")).unwrap()
}

#[test]
fn backend_payload_parses_and_ignores_extra_fields() {
    // The real agents_view shape: alias/tools/system_prompt_ref/model ride along.
    let json = r#"[
        {"id":"agent-crm-sync","display_name":"CRM Sync","alias":"synchro-clients",
         "tools":["google_sheets_list_spreadsheets"],"system_prompt_ref":"crm_sync_v1",
         "model":"gpt-5.4-mini"},
        {"id":"agent-notes","display_name":"Notes Helper"}
    ]"#;
    let parsed: Vec<AgentSummary> = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].id, "agent-crm-sync");
    assert_eq!(parsed[0].display_name, "CRM Sync");
    assert_eq!(parsed[0].alias, "synchro-clients");
    assert_eq!(parsed[1].alias, "");
}

#[test]
fn mark_installed_joins_backend_list_with_pinned_ids() {
    let summaries = vec![
        AgentSummary {
            id: "a".into(),
            display_name: "A".into(),
            alias: "run-a".into(),
        },
        AgentSummary {
            id: "b".into(),
            display_name: "B".into(),
            alias: String::new(),
        },
    ];
    let entries = mark_installed(summaries, &["b".to_string()]);
    assert_eq!(
        entries,
        vec![
            AgentEntry {
                id: "a".into(),
                name: "A".into(),
                alias: "run-a".into(),
                installed: false,
            },
            AgentEntry {
                id: "b".into(),
                name: "B".into(),
                alias: String::new(),
                installed: true,
            },
        ]
    );
}

#[tokio::test]
async fn no_backend_configured_is_a_clear_error() {
    if std::env::var("FLOWFLOW_BACKEND_URL").is_ok() {
        return; // an ambient override would legitimately configure a backend
    }
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    assert!(list_agents(&db).await.unwrap_err().contains("no backend"));
    assert!(ensure_installed(&db, "agent-x")
        .await
        .unwrap_err()
        .contains("no backend"));
}

#[test]
fn uninstall_drops_only_the_pinned_row() {
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap();
    let id = verified.manifest.id.clone();
    db.install_agent(&verified).unwrap();
    assert!(db.get_installed_agent(&id).is_some());

    uninstall(&db, &id).unwrap();
    assert!(db.get_installed_agent(&id).is_none());
    // Idempotent: removing an absent row is not an error.
    uninstall(&db, &id).unwrap();
}
