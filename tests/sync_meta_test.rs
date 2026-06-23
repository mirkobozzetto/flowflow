use flowflow::domain::{
    NewAttachment, NewFolder, NewNoteReminder, NewTextNote,
};
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db = Database::open_at(path).expect("open_at");
    (db, dir)
}

fn make_note(db: &Database, content: &str) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("note".into()),
        content: content.into(),
        tags: vec![],
    })
    .expect("create note")
    .id
}

// (origin_device, origin_seq, deleted, local version_vector counter)
fn meta(
    db: &Database,
    kind: &str,
    id: &str,
) -> Option<(String, i64, i64, i64)> {
    let conn = db.conn();
    let dev: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'sync_device_id'",
            [],
            |r| r.get(0),
        )
        .ok()?;
    conn.query_row(
        "SELECT origin_device, origin_seq, deleted,
                COALESCE(json_extract(version_vector, '$.\"' || ?3 || '\"'), 0)
         FROM sync_row_meta WHERE entity_kind = ?1 AND entity_id = ?2",
        rusqlite::params![kind, id, dev],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .ok()
}

fn sample_reminder(note_id: &str, hash: &str) -> NewNoteReminder {
    NewNoteReminder {
        note_id: note_id.into(),
        reminder_id: "ek-1".into(),
        backend: "eventkit".into(),
        intent_hash: hash.into(),
        due_year: Some(2026),
        due_month: Some(6),
        due_day: Some(9),
        due_hour: Some(10),
        due_minute: Some(0),
        is_all_day: false,
        tz_id: Some("Europe/Brussels".into()),
        recurrence: None,
    }
}

// --- T01: migration V10 ---------------------------------------------------

#[test]
fn test_migration_v10_creates_sync_tables() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    for table in [
        "sync_row_meta",
        "sync_seq",
        "sync_peers",
        "sync_conflicts",
        "chunks",
    ] {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = ?1",
                [table],
                |r| r.get(0),
            )
            .expect("query sqlite_master");
        assert_eq!(n, 1, "table {table} should exist after V10");
    }
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |r| r.get(0),
        )
        .expect("version");
    assert!(version >= 10, "V10 recorded, got {version}");
}

#[test]
fn test_v10_reopen_is_idempotent_and_nondestructive() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let note_id = {
        let db = Database::open_at(path.clone()).expect("open 1");
        make_note(&db, "survivor")
    };
    // Reopen the same file: migrate() must skip V10, data must survive.
    let db = Database::open_at(path).expect("open 2");
    assert!(
        db.get_note(&note_id).expect("get").is_some(),
        "note must survive a reopen across V10"
    );
}

// --- T03: INSERT/UPDATE tracking -----------------------------------------

#[test]
fn test_insert_note_tracks_local_version() {
    let (db, _dir) = open_test_db();
    let id = make_note(&db, "hello");
    let (origin, seq, deleted, count) =
        meta(&db, "note", &id).expect("meta exists for inserted note");
    assert!(!origin.is_empty(), "origin_device set");
    assert!(seq > 0, "origin_seq allocated, got {seq}");
    assert_eq!(deleted, 0, "fresh note not tombstoned");
    assert_eq!(count, 1, "local version_vector counter == 1 after insert");
}

#[test]
fn test_update_note_bumps_version_and_seq() {
    let (db, _dir) = open_test_db();
    let id = make_note(&db, "v1");
    let (_, seq1, _, count1) = meta(&db, "note", &id).unwrap();
    db.update_note(
        &id,
        &flowflow::domain::UpdateNote {
            title: None,
            content: Some("v2".into()),
            tags: None,
        },
    )
    .expect("update");
    let (_, seq2, _, count2) = meta(&db, "note", &id).unwrap();
    assert!(count2 > count1, "version counter bumps on update");
    assert!(seq2 > seq1, "origin_seq advances on update");
}

#[test]
fn test_origin_seq_is_unique_and_monotonic() {
    let (db, _dir) = open_test_db();
    let a = make_note(&db, "a");
    let b = make_note(&db, "b");
    db.update_note(
        &a,
        &flowflow::domain::UpdateNote {
            title: None,
            content: Some("a2".into()),
            tags: None,
        },
    )
    .unwrap();
    let conn = db.conn();
    let mut stmt = conn
        .prepare("SELECT origin_seq FROM sync_row_meta ORDER BY origin_seq")
        .unwrap();
    let seqs: Vec<i64> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .flatten()
        .collect();
    assert!(seqs.len() >= 2);
    for w in seqs.windows(2) {
        assert!(w[1] > w[0], "origin_seq must be strictly increasing/unique");
    }
    let _ = (a, b);
}

// --- T04: every write path tracks (incl. a fresh connection) -------------

#[test]
fn test_fresh_connection_still_tracks() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db1 = Database::open_at(path.clone()).expect("open 1");
    let id = make_note(&db1, "by db1");
    let (_, _, _, count1) = meta(&db1, "note", &id).unwrap();

    // A second Database handle = a different Connection (like an embed thread).
    let db2 = Database::open_at(path).expect("open 2");
    db2.update_note(
        &id,
        &flowflow::domain::UpdateNote {
            title: None,
            content: Some("by db2".into()),
            tags: None,
        },
    )
    .expect("update via db2");
    let (_, _, _, count2) = meta(&db2, "note", &id).unwrap();
    assert!(
        count2 > count1,
        "a write via a fresh connection must still bump the version (default=local)"
    );
}

#[test]
fn test_set_audio_transcription_is_tracked() {
    let (db, _dir) = open_test_db();
    let note_id = make_note(&db, "with audio");
    let audio = db.add_audio(&note_id, "a.wav", 1.0).expect("add audio");
    let (_, _, _, count_after_insert) =
        meta(&db, "note_audio", &audio.id).expect("audio meta");
    db.set_audio_transcription(&audio.id, "bonjour")
        .expect("set transcription");
    let (_, _, deleted, count_after_update) =
        meta(&db, "note_audio", &audio.id).unwrap();
    assert_eq!(deleted, 0);
    assert!(
        count_after_update > count_after_insert,
        "set_audio_transcription (an UPDATE) must bump the version"
    );
}

#[test]
fn test_notes_folders_link_tracked_and_tombstoned() {
    let (db, _dir) = open_test_db();
    let note_id = make_note(&db, "n");
    let folder = db
        .create_folder(&NewFolder {
            name: "F".into(),
            description: None,
            parent_id: None,
        })
        .expect("folder");
    db.add_note_to_folder(&note_id, &folder.id).expect("link");
    let link_id = format!("{}:{}", folder.id, note_id);
    let (_, _, deleted, count) =
        meta(&db, "notes_folders", &link_id).expect("link meta");
    assert_eq!(deleted, 0);
    assert_eq!(count, 1);

    db.remove_note_from_folder(&note_id, &folder.id)
        .expect("unlink");
    let (_, _, deleted2, _) = meta(&db, "notes_folders", &link_id).unwrap();
    assert_eq!(deleted2, 1, "unlink must tombstone the link");
}

// --- T05: tombstones on delete -------------------------------------------

#[test]
fn test_delete_note_tombstones_note_and_all_children() {
    let (db, _dir) = open_test_db();
    let note_id = make_note(&db, "doomed");
    let att = db
        .create_attachment(&NewAttachment {
            note_id: note_id.clone(),
            filename: "f.txt".into(),
            content_text: "x".into(),
        })
        .expect("attachment");
    let audio = db.add_audio(&note_id, "a.wav", 1.0).expect("audio");
    let reminder = db
        .add_note_reminder(&sample_reminder(&note_id, "h1"))
        .expect("reminder");
    let folder = db
        .create_folder(&NewFolder {
            name: "F".into(),
            description: None,
            parent_id: None,
        })
        .expect("folder");
    db.add_note_to_folder(&note_id, &folder.id).expect("link");

    db.delete_note(&note_id).expect("delete note");

    // Every entity carries a tombstone (deleted = 1).
    assert_eq!(meta(&db, "note", &note_id).unwrap().2, 1, "note tombstoned");
    assert_eq!(
        meta(&db, "attachment", &att.id).unwrap().2,
        1,
        "attachment tombstoned"
    );
    assert_eq!(
        meta(&db, "note_audio", &audio.id).unwrap().2,
        1,
        "audio tombstoned"
    );
    assert_eq!(
        meta(&db, "note_reminder", &reminder.id).unwrap().2,
        1,
        "reminder tombstoned"
    );
    let link_id = format!("{}:{}", folder.id, note_id);
    assert_eq!(
        meta(&db, "notes_folders", &link_id).unwrap().2,
        1,
        "link tombstoned"
    );

    // CASCADE still removed the rows physically.
    assert!(db.get_note(&note_id).unwrap().is_none());
    assert!(db.get_attachment(&att.id).unwrap().is_none());
}

#[test]
fn test_reminder_state_tombstone_maps_to_deleted() {
    let (db, _dir) = open_test_db();
    let note_id = make_note(&db, "n");
    let reminder = db
        .add_note_reminder(&sample_reminder(&note_id, "h1"))
        .expect("reminder");
    assert_eq!(meta(&db, "note_reminder", &reminder.id).unwrap().2, 0);

    db.set_reminder_state(&reminder.id, "tombstone")
        .expect("tombstone state");
    assert_eq!(
        meta(&db, "note_reminder", &reminder.id).unwrap().2,
        1,
        "state='tombstone' must map to meta deleted=1 (one notion of deleted)"
    );
}

// --- apply marker: triggers no-op on the sync connection ------------------

#[test]
fn test_apply_marker_makes_triggers_noop() {
    let (db, _dir) = open_test_db();
    // While "applying" a remote batch, local tracking must NOT fire.
    db.set_applying(true);
    let applied = make_note(&db, "remote-applied");
    db.set_applying(false);
    assert!(
        meta(&db, "note", &applied).is_none(),
        "a write while applying must NOT create local meta"
    );
    // Normal writes resume tracking once the flag is cleared.
    let local = make_note(&db, "local");
    assert!(
        meta(&db, "note", &local).is_some(),
        "tracking must resume after applying=false"
    );
}

// --- subfolder reparent (FK SET NULL) must be tracked via the AU trigger ---

#[test]
fn test_subfolder_reparent_on_parent_delete_is_tracked() {
    let (db, _dir) = open_test_db();
    let parent = db
        .create_folder(&NewFolder {
            name: "P".into(),
            description: None,
            parent_id: None,
        })
        .expect("parent");
    let child = db
        .create_folder(&NewFolder {
            name: "C".into(),
            description: None,
            parent_id: Some(parent.id.clone()),
        })
        .expect("child");
    let (_, seq_before, _, _) = meta(&db, "folder", &child.id).unwrap();

    db.delete_folder(&parent.id).expect("delete parent");

    // Child survives, reparented to root.
    let c = db
        .get_folder(&child.id)
        .expect("get child")
        .expect("child survives parent delete");
    assert!(c.parent_id.is_none(), "subfolder reparented to NULL");
    // And the reparent is tracked (recursive_triggers fires AFTER UPDATE).
    let (_, seq_after, deleted, _) = meta(&db, "folder", &child.id).unwrap();
    assert_eq!(deleted, 0, "reparented subfolder is not tombstoned");
    assert!(
        seq_after > seq_before,
        "subfolder reparent must bump its version (FK SET NULL fires the AU trigger)"
    );
}
