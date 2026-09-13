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
                stale: false,
            },
            AgentEntry {
                id: "b".into(),
                name: "B".into(),
                alias: String::new(),
                installed: true,
                stale: false,
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

#[test]
fn arm_binding_survives_uninstall_and_reinstall() {
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap();
    let id = verified.manifest.id.clone();
    db.install_agent(&verified).unwrap();
    let bound = r#"{"spreadsheet_id":["1AbCd"]}"#;
    db.set_agent_binding(&id, Some(bound)).unwrap();

    // Remove from the directory card: the pinned row goes, the binding is stashed.
    uninstall(&db, &id).unwrap();
    assert!(db.get_installed_agent(&id).is_none());
    assert!(db.get_agent_binding(&id).is_none());

    // Reinstall (the repo half of ensure_installed): the stash restores the armed sheets.
    db.install_agent(&verified).unwrap();
    flowflow::application::agent_directory::restore_binding_for_test(&db, &id);
    let restored = db.get_agent_binding(&id).unwrap();
    assert_eq!(restored["spreadsheet_id"][0], "1AbCd");

    // The stash is consumed: a later fresh install does not resurrect an old binding.
    db.set_agent_binding(&id, None).unwrap();
    flowflow::application::agent_directory::restore_binding_for_test(&db, &id);
    assert!(db.get_agent_binding(&id).is_none());
}

#[test]
fn served_entries_are_never_stale_and_ghosts_are_never_auto_removed() {
    let dir = tempdir().expect("tempdir");
    let db = open_db(&dir);
    // pin the fixture agent, then plant a ghost row under a stale id
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY)
        .expect("fixture verifies");
    db.install_agent(&verified).unwrap();
    let mut ghost = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap();
    ghost.manifest.id = "crm-sync".into();
    db.install_agent(&ghost).unwrap();

    // the served join never flags stale rows...
    let served = mark_installed(
        vec![AgentSummary {
            id: "agent-crm-sync".into(),
            display_name: "CRM Sync".into(),
            alias: "synchro-clients".into(),
        }],
        &["agent-crm-sync".into(), "crm-sync".into()],
    );
    assert!(served.iter().all(|e| !e.stale));

    // ...and nothing removes the ghost behind the user's back: the row (and any arm-time
    // binding it carries) survives until an explicit uninstall from the stale card.
    assert_eq!(db.list_installed_agents().len(), 2);
    uninstall(&db, "crm-sync").unwrap();
    assert_eq!(db.list_installed_agents().len(), 1);
    assert_eq!(db.list_installed_agents()[0].id, "agent-crm-sync");
}

#[tokio::test]
async fn v2_transport_verifies_and_installs_scoped_package() {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let manifest = json!({
        "schema_version":"2", "id":"scoped-install", "version":"1",
        "name":"Scoped", "alias":"scoped", "model":"test",
        "execution": {
            "required_connectors": [], "native_tools": ["create_note"],
            "governance": {
                "tools": [{"tool":"create_note","mode":"read_write","approval":"require_approval"}],
                "read_before_write": false, "deny_destructive": true
            },
            "orchestration": {"chains": {}}
        }
    });
    let digest = flowflow::domain::agent_manifest::digest_of(&manifest);
    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let package = json!({
        "manifest": manifest, "content_digest": digest,
        "signature": format!("ed25519:{}", B64.encode(signing.sign(digest.as_bytes()).to_bytes())),
        "signer_key_id":"dev-admin", "status":"published"
    }).to_string();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for _ in 0..1 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0u8; 8192];
            let n = stream.read(&mut request).await.unwrap();
            let head = String::from_utf8_lossy(&request[..n]);
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: bearer local-session"));
            let path = head
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap();
            assert_eq!(path, "/v2/agents/scoped-install/package");
            let (status, body) = ("200 OK", package.as_str());
            let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let dir = tempdir().unwrap();
    let db = open_db(&dir);
    db.set_setting("backend_base_url", &base).unwrap();
    db.set_setting("backend_session_token", "local-session")
        .unwrap();
    db.set_setting(
        "backend_session_expires_at",
        &(chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
    )
    .unwrap();
    let backend =
        flowflow::infrastructure::backend::BackendClient::from_db(&db).unwrap();
    let raw = backend
        .fetch_scoped_agent_package(&db, "scoped-install")
        .await
        .unwrap();
    let test_key =
        format!("ed25519:{}", B64.encode(signing.verifying_key().to_bytes()));
    let verified =
        flowflow::domain::scoped_agent_manifest::verify_scoped_package(
            &raw, &test_key,
        )
        .unwrap();
    db.install_scoped_agent("scoped-install", &verified)
        .unwrap();
    let row = db.get_installed_agent("scoped-install").unwrap();
    assert_eq!(row.content_digest, digest);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&row.manifest_json).unwrap()
            ["schema_version"],
        "2"
    );
    server.await.unwrap();
}
