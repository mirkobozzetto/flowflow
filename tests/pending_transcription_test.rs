use flowflow::db::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db = Database::open_at(path).expect("open_at");
    (db, dir)
}

#[test]
fn test_migration_v8_creates_pending_transcriptions_table() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name = 'pending_transcriptions'",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert_eq!(
        exists, 1,
        "pending_transcriptions table should exist after V8"
    );
}

#[test]
fn test_migration_v8_columns() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let mut stmt = conn
        .prepare("PRAGMA table_info(pending_transcriptions)")
        .expect("prepare pragma");
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>("name"))
        .expect("query")
        .filter_map(Result::ok)
        .collect();
    assert!(cols.contains(&"note_id".to_string()));
    assert!(cols.contains(&"transcription_id".to_string()));
    assert!(cols.contains(&"soniox_file_id".to_string()));
    assert!(cols.contains(&"created_at".to_string()));
    assert!(cols.contains(&"provider".to_string()));
    assert!(cols.contains(&"file_path".to_string()));
}

#[test]
fn test_migration_v11_old_rows_read_with_default_provider() {
    let (db, _dir) = open_test_db();

    db.add_pending_transcription("note-old", "tr-old", Some("file-old"))
        .expect("add");
    let rows = db.list_pending_transcriptions();
    assert_eq!(rows[0].provider, "soniox");
    assert_eq!(rows[0].file_path, None);
}

#[test]
fn test_local_pending_roundtrip() {
    let (db, _dir) = open_test_db();

    db.add_pending_local_transcription("note-l", "/tmp/audio.wav")
        .expect("add local");
    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].provider, "whisper_local");
    assert_eq!(rows[0].transcription_id, None);
    assert_eq!(rows[0].soniox_file_id, None);
    assert_eq!(rows[0].file_path.as_deref(), Some("/tmp/audio.wav"));

    db.add_pending_transcription("note-l", "tr-x", None)
        .expect("upsert to soniox");
    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].provider, "soniox");
    assert_eq!(rows[0].file_path, None);
}

#[test]
fn test_pending_transcription_crud() {
    let (db, _dir) = open_test_db();

    db.add_pending_transcription("note-1", "tr-1", Some("file-1"))
        .expect("add");
    db.add_pending_transcription("note-2", "tr-2", None)
        .expect("add none");

    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 2);

    let r1 = rows.iter().find(|r| r.note_id == "note-1").expect("note-1");
    assert_eq!(r1.transcription_id.as_deref(), Some("tr-1"));
    assert_eq!(r1.soniox_file_id.as_deref(), Some("file-1"));

    let r2 = rows.iter().find(|r| r.note_id == "note-2").expect("note-2");
    assert_eq!(r2.soniox_file_id, None);

    db.delete_pending_transcription("note-1").expect("delete");
    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].note_id, "note-2");
}

#[test]
fn test_pending_transcription_upsert_replaces() {
    let (db, _dir) = open_test_db();

    db.add_pending_transcription("note-1", "tr-old", Some("file-old"))
        .expect("add");
    db.add_pending_transcription("note-1", "tr-new", Some("file-new"))
        .expect("upsert");

    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 1, "upsert must not duplicate the note_id");
    assert_eq!(rows[0].transcription_id.as_deref(), Some("tr-new"));
    assert_eq!(rows[0].soniox_file_id.as_deref(), Some("file-new"));
}
