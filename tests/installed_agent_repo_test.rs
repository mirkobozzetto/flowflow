// installed_agents persistence: pin (verify -> install), reload byte-identical, upsert repins, active
// toggles, and the stored manifest still hashes to its pinned digest.

use flowflow::application::connector_module::{
    FIXTURE_AGENT_ID, FIXTURE_PACKAGE,
};
use flowflow::domain::agent_manifest::{
    digest_of_stored, verify_package, ADMIN_PUBKEY,
};
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");
    (db, dir)
}

#[test]
fn migration_v14_creates_installed_agents_table() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name = 'installed_agents'",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert_eq!(exists, 1);
}

#[test]
fn install_then_reload_is_byte_identical_and_pinned() {
    let (db, _dir) = open_test_db();
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap();
    db.install_agent(&verified).expect("install");

    let row = db
        .get_installed_agent(FIXTURE_AGENT_ID)
        .expect("agent is pinned");
    assert_eq!(row.id, FIXTURE_AGENT_ID);
    assert_eq!(row.version, "1.0.0");
    assert_eq!(row.content_digest, verified.content_digest);
    assert_eq!(row.manifest_json, verified.manifest_json);
    assert!(row.active);

    // The pinned digest is the integrity check on the stored row.
    assert_eq!(
        digest_of_stored(&row.manifest_json).unwrap(),
        row.content_digest
    );
}

#[test]
fn reinstall_upserts_one_row() {
    let (db, _dir) = open_test_db();
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap();
    db.install_agent(&verified).unwrap();
    db.install_agent(&verified).unwrap();
    assert_eq!(db.list_installed_agents().len(), 1);
}

#[test]
fn active_toggles() {
    let (db, _dir) = open_test_db();
    let verified = verify_package(FIXTURE_PACKAGE, ADMIN_PUBKEY).unwrap();
    db.install_agent(&verified).unwrap();

    db.set_agent_active(FIXTURE_AGENT_ID, false).unwrap();
    assert!(!db.get_installed_agent(FIXTURE_AGENT_ID).unwrap().active);
    db.set_agent_active(FIXTURE_AGENT_ID, true).unwrap();
    assert!(db.get_installed_agent(FIXTURE_AGENT_ID).unwrap().active);
}
