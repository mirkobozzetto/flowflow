use flowflow::domain::NewTextNote;
use flowflow::infrastructure::persistence::Database;
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

// A space row surviving "delete my data" means a stale cursor that makes the
// next pull skip everything, and a purge queue naming notes that no longer exist.
#[test]
fn wipe_clears_space_rows_and_the_purge_queue() {
    let (db, _dir) = open_test_db();
    db.upsert_space("space-1", "Team", false).expect("space");
    db.queue_purge("gone-note", "note").expect("queue");

    db.wipe_local_content().expect("wipe");

    assert!(db.list_spaces().unwrap().is_empty(), "spaces wiped");
    assert!(db.pending_purges().unwrap().is_empty(), "purge queue wiped");
}

#[test]
fn wipe_clears_note_links_and_pending_publishes() {
    let (db, _dir) = open_test_db();
    let first = db
        .create_text_note(&NewTextNote {
            title: Some("first".into()),
            content: String::new(),
            tags: vec![],
        })
        .expect("create first note");
    let second = db
        .create_text_note(&NewTextNote {
            title: Some("second".into()),
            content: String::new(),
            tags: vec![],
        })
        .expect("create second note");
    db.replace_note_links(&first.id, &[(second.id, 1.0)])
        .expect("link notes");
    db.upsert_space("space-1", "Team", false).expect("space");
    db.stage_note_publish(&first.id, "space-1", "remote-1", Some("author"))
        .expect("stage publish");

    db.wipe_local_content().expect("wipe");

    assert!(!db.has_note_links(&first.id).expect("read links"));
    assert!(db.note_publish_state(&first.id).is_none());
}
