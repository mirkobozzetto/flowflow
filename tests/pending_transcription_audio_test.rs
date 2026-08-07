use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open_at");
    (db, dir)
}

#[test]
fn migration_v21_adds_the_audio_id_column() {
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
    assert!(cols.contains(&"audio_id".to_string()));
}

#[test]
fn a_local_row_round_trips_its_audio_id() {
    let (db, _dir) = open_test_db();

    db.add_pending_local_transcription("note-1", "/tmp/a.wav", Some("aud-1"))
        .expect("add local");

    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].provider, "whisper_local");
    assert_eq!(rows[0].audio_id.as_deref(), Some("aud-1"));
}

#[test]
fn a_soniox_row_round_trips_its_audio_id() {
    let (db, _dir) = open_test_db();

    db.add_pending_transcription("note-2", "tr-2", Some("fid"), Some("aud-2"))
        .expect("add soniox");

    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].provider, "soniox");
    assert_eq!(rows[0].audio_id.as_deref(), Some("aud-2"));
}

#[test]
fn an_import_round_trips_a_missing_audio_id() {
    let (db, _dir) = open_test_db();

    db.add_pending_local_transcription("note-3", "/tmp/b.wav", None)
        .expect("add local");
    db.add_pending_transcription("note-4", "tr-4", None, None)
        .expect("add soniox");

    let rows = db.list_pending_transcriptions();
    assert!(rows.iter().all(|r| r.audio_id.is_none()));
}

#[test]
fn the_upsert_overwrites_a_previous_audio_id() {
    let (db, _dir) = open_test_db();

    db.add_pending_local_transcription("note-1", "/tmp/a.wav", Some("aud-1"))
        .expect("first");
    db.add_pending_local_transcription("note-1", "/tmp/a.wav", Some("aud-9"))
        .expect("second");

    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 1, "upsert must not duplicate the note_id");
    assert_eq!(rows[0].audio_id.as_deref(), Some("aud-9"));

    db.add_pending_local_transcription("note-1", "/tmp/a.wav", None)
        .expect("third");

    let rows = db.list_pending_transcriptions();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].audio_id, None);
}
