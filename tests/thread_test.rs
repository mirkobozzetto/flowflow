use flowflow::db::Database;
use flowflow::models::{NewTextNote, NewThread, UpdateThread};
use flowflow::services::backup;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let (db, dir, _path) = open_test_db_with_path();
    (db, dir)
}

fn open_test_db_with_path() -> (Database, tempfile::TempDir, std::path::PathBuf)
{
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db = Database::open_at(path.clone()).expect("open_at");
    (db, dir, path)
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

fn meta(db: &Database, kind: &str, id: &str) -> Option<(i64, i64)> {
    let conn = db.conn();
    let dev: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'sync_device_id'",
            [],
            |r| r.get(0),
        )
        .ok()?;
    conn.query_row(
        "SELECT deleted,
                COALESCE(json_extract(version_vector, '$.\"' || ?3 || '\"'), 0)
         FROM sync_row_meta WHERE entity_kind = ?1 AND entity_id = ?2",
        rusqlite::params![kind, id, dev],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .ok()
}

#[test]
fn migration_v13_creates_threads_and_thread_id() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();

    let threads_table: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='threads'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(threads_table, 1, "threads table must exist");

    let has_col: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('notes') WHERE name='thread_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(has_col, 1, "notes.thread_id column must exist");

    let recorded: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM _migrations WHERE version = 13",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(recorded, 1, "V13 must be recorded");
}

#[test]
fn existing_notes_are_flat_after_migration() {
    let (db, _dir) = open_test_db();
    let nid = make_note(&db, "flat note");
    let note = db.get_note(&nid).unwrap().unwrap();
    assert_eq!(note.thread_id, None);
}

#[test]
fn thread_crud_and_membership() {
    let (db, _dir) = open_test_db();
    let folder = db
        .create_folder(&flowflow::models::NewFolder {
            name: "Idées".into(),
            description: None,
            parent_id: None,
        })
        .unwrap();

    let thread = db
        .create_thread(&NewThread {
            title: "My thread".into(),
            folder_id: Some(folder.id.clone()),
        })
        .unwrap();
    assert_eq!(thread.title, "My thread");
    assert_eq!(thread.folder_id, Some(folder.id.clone()));

    assert_eq!(db.list_threads().unwrap().len(), 1);

    let n1 = make_note(&db, "first");
    let n2 = make_note(&db, "second");
    db.add_note_to_thread(&n1, &thread.id).unwrap();
    db.add_note_to_thread(&n2, &thread.id).unwrap();

    let members = db.list_thread_notes(&thread.id).unwrap();
    assert_eq!(members.len(), 2);
    let ids: Vec<&str> = members.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&n1.as_str()) && ids.contains(&n2.as_str()));

    db.update_thread(
        &thread.id,
        &UpdateThread {
            title: Some("Renamed".into()),
            folder_id: Some(None),
        },
    )
    .unwrap();
    let updated = db.get_thread(&thread.id).unwrap().unwrap();
    assert_eq!(updated.title, "Renamed");
    assert_eq!(updated.folder_id, None);

    db.remove_note_from_thread(&n1).unwrap();
    assert_eq!(db.list_thread_notes(&thread.id).unwrap().len(), 1);
    assert_eq!(db.get_note(&n1).unwrap().unwrap().thread_id, None);
}

#[test]
fn create_text_note_in_thread_sets_membership() {
    let (db, _dir) = open_test_db();
    let thread = db
        .create_thread(&NewThread {
            title: "T".into(),
            folder_id: None,
        })
        .unwrap();
    let note = db
        .create_text_note_in_thread(
            &NewTextNote {
                title: Some("entry".into()),
                content: "hello".into(),
                tags: vec![],
            },
            &thread.id,
        )
        .unwrap();
    assert_eq!(note.thread_id, Some(thread.id.clone()));
    assert_eq!(db.list_thread_notes(&thread.id).unwrap().len(), 1);
}

#[test]
fn list_root_notes_excludes_members_includes_dangling() {
    let (db, _dir) = open_test_db();
    let flat = make_note(&db, "flat");
    let thread = db
        .create_thread(&NewThread {
            title: "T".into(),
            folder_id: None,
        })
        .unwrap();
    let m1 = make_note(&db, "m1");
    let m2 = make_note(&db, "m2");
    db.add_note_to_thread(&m1, &thread.id).unwrap();
    db.add_note_to_thread(&m2, &thread.id).unwrap();

    let root: Vec<String> = db
        .list_root_notes()
        .unwrap()
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert!(root.contains(&flat), "flat note must be in root feed");
    assert!(
        !root.contains(&m1) && !root.contains(&m2),
        "thread members must not be in root feed"
    );

    db.delete_thread(&thread.id).unwrap();
    let root2: Vec<String> = db
        .list_root_notes()
        .unwrap()
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert!(
        root2.contains(&m1) && root2.contains(&m2),
        "after thread delete the members return to the root feed"
    );
}

#[test]
fn thread_needs_two_members_to_be_a_thread() {
    let (db, _dir) = open_test_db();
    let thread = db
        .create_thread(&NewThread {
            title: "T".into(),
            folder_id: None,
        })
        .unwrap();
    let m1 = make_note(&db, "m1");
    db.add_note_to_thread(&m1, &thread.id).unwrap();

    assert!(db.list_feed_threads().unwrap().is_empty());
    let root: Vec<String> = db
        .list_root_notes()
        .unwrap()
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert!(root.contains(&m1), "lone member shows as a flat note");

    let m2 = make_note(&db, "m2");
    db.add_note_to_thread(&m2, &thread.id).unwrap();

    assert_eq!(db.list_feed_threads().unwrap().len(), 1);
    let root2: Vec<String> = db
        .list_root_notes()
        .unwrap()
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert!(!root2.contains(&m1) && !root2.contains(&m2));
}

#[test]
fn collapse_singleton_threads_reverts_lone_members() {
    let (db, _dir) = open_test_db();
    let thread = db
        .create_thread(&NewThread {
            title: "T".into(),
            folder_id: None,
        })
        .unwrap();
    let m1 = make_note(&db, "m1");
    db.add_note_to_thread(&m1, &thread.id).unwrap();

    db.collapse_singleton_threads().unwrap();

    assert!(
        db.get_thread(&thread.id).unwrap().is_none(),
        "singleton thread removed"
    );
    assert_eq!(
        db.get_note(&m1).unwrap().unwrap().thread_id,
        None,
        "lone member reverts to flat"
    );
}

#[test]
fn delete_thread_keeps_members_and_converges() {
    let (db, _dir) = open_test_db();
    let thread = db
        .create_thread(&NewThread {
            title: "T".into(),
            folder_id: None,
        })
        .unwrap();
    assert!(meta(&db, "thread", &thread.id).is_some(), "thread tracked");

    let m1 = make_note(&db, "m1");
    let m2 = make_note(&db, "m2");
    db.add_note_to_thread(&m1, &thread.id).unwrap();
    db.add_note_to_thread(&m2, &thread.id).unwrap();
    let before = meta(&db, "note", &m1).unwrap().1;

    db.delete_thread(&thread.id).unwrap();

    assert!(db.get_thread(&thread.id).unwrap().is_none());
    assert_eq!(db.get_note(&m1).unwrap().unwrap().thread_id, None);
    assert_eq!(db.get_note(&m2).unwrap().unwrap().thread_id, None);

    let (deleted, _) = meta(&db, "thread", &thread.id).unwrap();
    assert_eq!(deleted, 1, "thread must be tombstoned");

    let (m1_deleted, m1_vv) = meta(&db, "note", &m1).unwrap();
    assert_eq!(m1_deleted, 0, "member note stays alive");
    assert!(
        m1_vv > before,
        "member note version must bump so it travels"
    );
}

#[test]
fn backup_counts_include_threads() {
    let (db, dir, db_path) = open_test_db_with_path();
    db.create_thread(&NewThread {
        title: "T".into(),
        folder_id: None,
    })
    .unwrap();
    db.checkpoint_wal().ok();

    let staging = dir.path().join("export_staging");
    let snapshot =
        backup::create_scrubbed_snapshot(&db_path, &staging).expect("snapshot");
    let counts = backup::snapshot_counts(&snapshot).expect("counts");
    assert_eq!(counts.threads, 1, "backup must count threads");
}
