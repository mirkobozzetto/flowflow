// V23 author_device: the backfill credits sync_row_meta.origin_device, falls
// back to the local device id, and NEVER rewrites sync meta (the notes AFTER
// UPDATE trigger must stay silent under the apply guard). Reopening a
// schema-22 file with the v23 binary is exactly the backup-restore path
// (application/backup/validate.rs re-runs pending migrations the same way).

use flowflow::domain::NewTextNote;
use flowflow::infrastructure::persistence::Database;

fn snapshot_meta(db: &Database) -> Vec<(String, String, String, String, i64)> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(
            "SELECT entity_kind, entity_id, version_vector, origin_device,
                    origin_seq
             FROM sync_row_meta ORDER BY entity_kind, entity_id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap();
    rows.map(Result::unwrap).collect()
}

// Rewind the database to schema 22: drop the added columns and forget the
// migration records, so the next open re-runs V23+ against a realistic
// pre-upgrade file.
fn rewind_to_v22(db: &Database) {
    let conn = db.conn();
    // Every migration from 23 up has to be undone, not just V23's column:
    // deleting the records replays all of them, and an ALTER that re-adds an
    // existing column fails the whole batch.
    conn.execute_batch(
        "ALTER TABLE notes DROP COLUMN author_device;
         ALTER TABLE sync_peers DROP COLUMN name;
         DROP INDEX IF EXISTS idx_folders_space;
         DROP INDEX IF EXISTS idx_notes_space;
         DROP INDEX IF EXISTS idx_notes_remote;
         DROP INDEX IF EXISTS idx_folders_remote;
         ALTER TABLE folders DROP COLUMN space_id;
         ALTER TABLE folders DROP COLUMN remote_id;
         ALTER TABLE folders DROP COLUMN mode;
         ALTER TABLE notes DROP COLUMN space_id;
         ALTER TABLE notes DROP COLUMN remote_id;
         ALTER TABLE notes DROP COLUMN author_ref;
         DROP TABLE IF EXISTS spaces;
         DROP TABLE IF EXISTS pending_purge;
         DELETE FROM _migrations WHERE version >= 23;",
    )
    .unwrap();
}

#[test]
fn v23_backfill_credits_origin_device_and_leaves_meta_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("flowflow.db");
    let before;
    {
        let db = Database::open_at(path.clone()).unwrap();
        db.create_text_note(&NewTextNote {
            title: Some("mine".into()),
            content: "written here".into(),
            tags: vec![],
        })
        .unwrap();
        // A note authored by a peer: meta carries the foreign origin_device.
        db.set_applying(true);
        db.conn()
            .execute_batch(
                "INSERT INTO notes (id, note_type, title, content, tags,
                     created_at, modified_at)
                 VALUES ('peer-note', 'text', 'theirs', 'from a peer', '[]',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 INSERT INTO sync_row_meta (entity_kind, entity_id,
                     version_vector, origin_device, origin_seq)
                 VALUES ('note', 'peer-note', '{}', 'peer-device-uuid', 7);",
            )
            .unwrap();
        db.set_applying(false);
        rewind_to_v22(&db);
        before = snapshot_meta(&db);
    }

    let db = Database::open_at(path.clone()).unwrap();
    assert_eq!(
        snapshot_meta(&db),
        before,
        "V23 backfill must not rewrite sync_row_meta"
    );
    drop(db);

    // Second reopen proves the recorded migration does not re-run.
    let db = Database::open_at(path).unwrap();
    assert_eq!(snapshot_meta(&db), before, "meta must survive reopen");

    let local: String = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'sync_device_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let author = |id: &str| -> Option<String> {
        db.conn()
            .query_row(
                "SELECT author_device FROM notes WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap()
    };
    let mine: String = db
        .conn()
        .query_row("SELECT id FROM notes WHERE id != 'peer-note'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(author(&mine).as_deref(), Some(local.as_str()));
    assert_eq!(author("peer-note").as_deref(), Some("peer-device-uuid"));
}

#[test]
fn v23_backfill_falls_back_to_local_device_without_meta() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("flowflow.db");
    {
        let db = Database::open_at(path.clone()).unwrap();
        db.set_applying(true);
        db.conn()
            .execute_batch(
                "INSERT INTO notes (id, note_type, title, content, tags,
                     created_at, modified_at)
                 VALUES ('orphan', 'text', 'old', 'pre-sync row', '[]',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 DELETE FROM sync_row_meta
                     WHERE entity_kind = 'note' AND entity_id = 'orphan';",
            )
            .unwrap();
        db.set_applying(false);
        rewind_to_v22(&db);
    }

    let db = Database::open_at(path).unwrap();
    let local: String = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'sync_device_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let author: Option<String> = db
        .conn()
        .query_row(
            "SELECT author_device FROM notes WHERE id = 'orphan'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(author.as_deref(), Some(local.as_str()));
}

#[test]
fn new_notes_are_stamped_with_the_local_device() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("flowflow.db")).unwrap();
    let note = db
        .create_text_note(&NewTextNote {
            title: None,
            content: "fresh".into(),
            tags: vec![],
        })
        .unwrap();
    let local: String = db
        .conn()
        .query_row(
            "SELECT value FROM settings WHERE key = 'sync_device_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(note.author_device.as_deref(), Some(local.as_str()));
}
