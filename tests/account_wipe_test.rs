use flowflow::db::Database;
use flowflow::models::NewTextNote;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db = Database::open_at(path).expect("open_at");
    (db, dir)
}

// Q1.6 "delete my data": wipe_local_content drops user notes but preserves settings (device identity,
// keys) so the app still boots after the wipe.
#[test]
fn wipe_clears_notes_keeps_settings() {
    let (db, _dir) = open_test_db();
    db.create_text_note(&NewTextNote {
        title: Some("keep me".into()),
        content: "secret note".into(),
        tags: vec![],
    })
    .expect("create note");
    db.set_setting("openai_api_key", "sk-test").expect("set");

    assert_eq!(db.list_notes().unwrap().len(), 1);

    let audio_paths = db.wipe_local_content().expect("wipe");
    assert!(audio_paths.is_empty(), "no audio was created");

    assert_eq!(db.list_notes().unwrap().len(), 0, "notes wiped");
    assert_eq!(
        db.get_setting("openai_api_key").as_deref(),
        Some("sk-test"),
        "settings (identity/keys) survive the wipe"
    );
}
