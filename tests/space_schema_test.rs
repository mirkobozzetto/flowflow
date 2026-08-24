// V26 spaces schema (proposal 0002, T07). Three things must hold together, and
// only a test holds them: the migration applies on a real V25 file, the space
// columns of `folders` and `notes` are declared in the sync catalog (a column
// missing from that fixed list never travels between the devices of one
// account), and `spaces` / `pending_purge` stay OUT of it (a pull cursor and a
// purge intent are device-local).

use flowflow::infrastructure::persistence::Database;

fn table_columns(db: &Database, table: &str) -> Vec<String> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap();
    let rows = stmt.query_map([], |r| r.get::<_, String>(1)).unwrap();
    rows.map(Result::unwrap).collect()
}

// Rewind to schema 25: drop the V26 columns and tables, forget the record, so
// the next open re-runs V26 against a realistic pre-upgrade file. Same shape as
// the V23 rewind in author_device_test.
fn rewind_to_v25(db: &Database) {
    let conn = db.conn();
    conn.execute_batch(
        "DROP INDEX IF EXISTS idx_folders_space;
         DROP INDEX IF EXISTS idx_notes_space;
         DROP INDEX IF EXISTS idx_notes_remote;
         DROP INDEX IF EXISTS idx_folders_remote;
         ALTER TABLE folders DROP COLUMN space_id;
         ALTER TABLE folders DROP COLUMN remote_id;
         ALTER TABLE folders DROP COLUMN mode;
         ALTER TABLE notes DROP COLUMN space_id;
         ALTER TABLE notes DROP COLUMN remote_id;
         ALTER TABLE notes DROP COLUMN author_ref;
         DROP TABLE spaces;
         DROP TABLE pending_purge;
         DELETE FROM _migrations WHERE version >= 26;",
    )
    .unwrap();
}

#[test]
fn v26_applies_on_a_v25_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("flowflow.db");
    {
        let db = Database::open_at(path.clone()).unwrap();
        rewind_to_v25(&db);
    }

    let db = Database::open_at(path).unwrap();
    let folders = table_columns(&db, "folders");
    let notes = table_columns(&db, "notes");
    for col in ["space_id", "remote_id", "mode"] {
        assert!(folders.contains(&col.to_string()), "folders.{col} missing");
    }
    for col in ["space_id", "remote_id", "author_ref"] {
        assert!(notes.contains(&col.to_string()), "notes.{col} missing");
    }
    assert!(!table_columns(&db, "spaces").is_empty());
    assert!(!table_columns(&db, "pending_purge").is_empty());

    let head: i64 = db
        .conn()
        .query_row("SELECT MAX(version) FROM _migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(head, 26);
}

#[test]
fn space_columns_travel_and_cursors_do_not() {
    let synced = flowflow::infrastructure::sync::protocol::synced_columns();

    let folder = synced.get("folder").expect("folder kind");
    for col in ["space_id", "remote_id", "mode"] {
        assert!(
            folder.contains(&col),
            "folder.{col} absent from the sync catalog: it would never reach \
             the other devices of the account"
        );
    }
    let note = synced.get("note").expect("note kind");
    for col in ["space_id", "remote_id", "author_ref"] {
        assert!(
            note.contains(&col),
            "note.{col} absent from the sync catalog"
        );
    }

    for kind in ["space", "spaces", "pending_purge"] {
        assert!(
            !synced.contains_key(kind),
            "{kind} must stay device-local: a pull cursor and a purge intent \
             are meaningless on another device"
        );
    }
}
