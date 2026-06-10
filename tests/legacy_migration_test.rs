#![cfg(not(target_os = "ios"))]

use flowflow::db::migrate_legacy_temp_data;
use rusqlite::Connection;
use tempfile::tempdir;

fn seed_legacy_db(dir: &std::path::Path) -> Connection {
    let conn = Connection::open(dir.join("flowflow.db")).expect("open legacy");
    conn.pragma_update(None, "journal_mode", "WAL").expect("wal");
    conn.execute("CREATE TABLE notes (id TEXT PRIMARY KEY, content TEXT)", [])
        .expect("schema");
    conn.execute(
        "INSERT INTO notes VALUES ('n1', 'written through the wal')",
        [],
    )
    .expect("insert");
    conn
}

#[test]
fn migration_carries_wal_resident_writes() {
    let legacy = tempdir().expect("legacy dir");
    let new_dir = tempdir().expect("new dir");
    let held_open = seed_legacy_db(legacy.path());
    assert!(
        legacy.path().join("flowflow.db-wal").exists(),
        "precondition: writes must sit in the wal"
    );

    migrate_legacy_temp_data(legacy.path(), new_dir.path());
    drop(held_open);

    let migrated =
        Connection::open(new_dir.path().join("flowflow.db")).expect("open");
    let content: String = migrated
        .query_row("SELECT content FROM notes WHERE id = 'n1'", [], |r| {
            r.get(0)
        })
        .expect("wal-resident row must survive the migration");
    assert_eq!(content, "written through the wal");
    assert!(
        !new_dir.path().join("flowflow.db-wal").exists(),
        "migrated image must be a single checkpointed file, no torn wal"
    );
    assert!(
        legacy.path().join("flowflow.db").exists(),
        "legacy store stays untouched as fallback"
    );
}

#[test]
fn migration_cleans_stale_staging_from_a_crash() {
    let legacy = tempdir().expect("legacy dir");
    let new_dir = tempdir().expect("new dir");
    let _held_open = seed_legacy_db(legacy.path());
    std::fs::write(new_dir.path().join("flowflow.db.migrating"), b"torn")
        .expect("plant stale staging");

    migrate_legacy_temp_data(legacy.path(), new_dir.path());

    assert!(
        !new_dir.path().join("flowflow.db.migrating").exists(),
        "stale staging file must be cleared"
    );
    let migrated =
        Connection::open(new_dir.path().join("flowflow.db")).expect("open");
    let count: i64 = migrated
        .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
        .expect("count");
    assert_eq!(count, 1, "migration must complete despite the stale staging");
}

#[test]
fn migration_never_overwrites_an_existing_target() {
    let legacy = tempdir().expect("legacy dir");
    let new_dir = tempdir().expect("new dir");
    let _held_open = seed_legacy_db(legacy.path());
    std::fs::write(new_dir.path().join("flowflow.db"), b"existing store")
        .expect("plant target");

    migrate_legacy_temp_data(legacy.path(), new_dir.path());

    let bytes =
        std::fs::read(new_dir.path().join("flowflow.db")).expect("read");
    assert_eq!(
        bytes, b"existing store",
        "an existing target must never be touched"
    );
}
