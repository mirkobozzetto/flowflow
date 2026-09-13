use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signer, SigningKey};
use flowflow::domain::agent_manifest::{
    digest_of, verify_package, ADMIN_PUBKEY,
};
use flowflow::domain::scoped_agent_manifest::{
    parse_scoped_manifest, verify_scoped_package,
};
use flowflow::infrastructure::persistence::Database;
use serde_json::{json, Value};

fn manifest() -> Value {
    json!({"schema_version":"2","id":"native-reader","version":"1.0.0","name":"Reader","model":"test",
        "metadata":{"custom_description":"kept byte-for-byte"},
        "execution":{"required_connectors":[],"native_tools":["search_notes"],
        "governance":{"tools":[{"tool":"search_notes","mode":"read_only"}]},
        "orchestration":{"chains":{}}}})
}
fn package(manifest: &Value) -> (String, String) {
    let key = SigningKey::from_bytes(&[11u8; 32]);
    let digest = digest_of(manifest);
    let signature = format!(
        "ed25519:{}",
        B64.encode(key.sign(digest.as_bytes()).to_bytes())
    );
    (json!({"manifest":manifest,"content_digest":digest,"signature":signature}).to_string(),
        format!("ed25519:{}", B64.encode(key.verifying_key().to_bytes())))
}

#[test]
fn signed_native_package_installs_and_reloads_original_bytes_after_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("test.db");
    let (raw, key) = package(&manifest());
    assert!(
        verify_package(&raw, &key).is_err(),
        "legacy reader stays schema-1-only"
    );
    let verified = verify_scoped_package(&raw, &key).unwrap();
    {
        let db = Database::open_at(path.clone()).unwrap();
        assert!(db.install_scoped_agent("wrong-id", &verified).is_err());
        db.install_scoped_agent("native-reader", &verified).unwrap();
    }
    let db = Database::open_at(path.clone()).unwrap();
    let row = db.get_installed_agent("native-reader").unwrap();
    assert_eq!(row.manifest_json, verified.canonical());
    assert_eq!(row.content_digest, verified.digest());
    assert!(db.get_agent_binding("native-reader").is_none());
    assert_eq!(
        db.load_scoped_agent("native-reader")
            .unwrap()
            .execution
            .native_tools,
        vec!["search_notes"]
    );
    db.set_agent_active("native-reader", false).unwrap();
    assert!(db.load_scoped_agent("native-reader").is_err());
}

#[test]
fn new_installation_cannot_migrate_an_existing_legacy_pin_or_binding() {
    let temp = tempfile::tempdir().unwrap();
    let db = Database::open_at(temp.path().join("test.db")).unwrap();
    let legacy = verify_package(
        flowflow::application::connector_module::FIXTURE_PACKAGE,
        ADMIN_PUBKEY,
    )
    .unwrap();
    db.install_agent(&legacy).unwrap();
    let id = &legacy.manifest.id;
    db.set_agent_binding(id, Some(r#"{"spreadsheet_id":"legacy-sheet"}"#))
        .unwrap();
    let before = db.get_installed_agent(id).unwrap();
    let mut value = manifest();
    value["id"] = json!(id);
    let (raw, key) = package(&value);
    let new = verify_scoped_package(&raw, &key).unwrap();
    assert!(db.install_scoped_agent(id, &new).is_err());
    let after = db.get_installed_agent(id).unwrap();
    assert_eq!(before.manifest_json, after.manifest_json);
    assert_eq!(before.content_digest, after.content_digest);
    assert_eq!(
        db.get_agent_binding(id),
        Some(json!({"spreadsheet_id":"legacy-sheet"}))
    );
}

#[test]
fn unknown_execution_fields_and_invalid_native_consent_fail_closed() {
    for mutation in 0..6 {
        let mut value = manifest();
        match mutation {
            0 => value["schema_version"] = json!("3"),
            1 => value["governance"] = json!({"tools":[]}),
            2 => {
                value["execution"]["governance"]["allow_everything"] =
                    json!(true)
            }
            3 => {
                value["execution"]["governance"]["tools"][0]["silent_write"] =
                    json!(true)
            }
            4 => value["execution"]["native_tools"] = json!(["unknown"]),
            _ => {
                value["execution"]["native_tools"] = json!(["create_note"]);
                value["execution"]["governance"]["tools"] =
                    json!([{"tool":"create_note","mode":"read_write"}]);
            }
        }
        assert!(
            parse_scoped_manifest(&value.to_string()).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn tampered_envelope_or_stored_digest_never_loads() {
    let (raw, key) = package(&manifest());
    let mut changed: Value = serde_json::from_str(&raw).unwrap();
    changed["manifest"]["name"] = json!("changed");
    assert!(verify_scoped_package(&changed.to_string(), &key).is_err());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("test.db");
    let db = Database::open_at(path.clone()).unwrap();
    let verified = verify_scoped_package(&raw, &key).unwrap();
    db.install_scoped_agent("native-reader", &verified).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("UPDATE installed_agents SET content_digest = 'wrong' WHERE id = 'native-reader'",[]).unwrap();
    assert!(db.load_scoped_agent("native-reader").is_err());
}
