use flowflow::domain::{NewNoteReminder, NewTextNote, ReminderIntent};
use flowflow::infrastructure::persistence::Database;
use tempfile::tempdir;

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flowflow_test.db");
    let db = Database::open_at(path).expect("open_at");
    (db, dir)
}

fn make_note(db: &Database) -> String {
    db.create_text_note(&NewTextNote {
        title: Some("note".into()),
        content: "content".into(),
        tags: vec![],
    })
    .expect("create note")
    .id
}

fn sample(note_id: &str, hash: &str) -> NewNoteReminder {
    NewNoteReminder {
        note_id: note_id.into(),
        reminder_id: "ek-123".into(),
        backend: "eventkit".into(),
        intent_hash: hash.into(),
        due_year: Some(2026),
        due_month: Some(6),
        due_day: Some(3),
        due_hour: Some(15),
        due_minute: Some(30),
        is_all_day: false,
        tz_id: Some("Europe/Brussels".into()),
        recurrence: None,
    }
}

#[test]
fn test_migration_v9_creates_note_reminders_table() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name = 'note_reminders'",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert_eq!(exists, 1, "note_reminders table should exist after V9");
}

#[test]
fn test_migration_v9_columns() {
    let (db, _dir) = open_test_db();
    let conn = db.conn();
    let mut stmt = conn
        .prepare("PRAGMA table_info(note_reminders)")
        .expect("prepare pragma");
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>("name"))
        .expect("query")
        .filter_map(Result::ok)
        .collect();
    for c in [
        "id",
        "note_id",
        "reminder_id",
        "backend",
        "intent_hash",
        "due_year",
        "due_minute",
        "is_all_day",
        "tz_id",
        "recurrence",
        "state",
        "created_at",
    ] {
        assert!(cols.contains(&c.to_string()), "missing column {c}");
    }
}

#[test]
fn test_note_reminder_crud() {
    let (db, _dir) = open_test_db();
    let note_id = make_note(&db);

    let created = db.add_note_reminder(&sample(&note_id, "h1")).expect("add");
    assert_eq!(created.state, "active");
    assert!(!created.id.is_empty());

    let rows = db.reminders_for_note(&note_id);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].reminder_id, "ek-123");
    assert_eq!(rows[0].due_hour, Some(15));
    assert_eq!(rows[0].tz_id.as_deref(), Some("Europe/Brussels"));

    db.delete_note_reminder(&created.id).expect("delete");
    assert!(db.reminders_for_note(&note_id).is_empty());
}

#[test]
fn test_anti_doublon_unique_intent_hash() {
    let (db, _dir) = open_test_db();
    let note_id = make_note(&db);

    db.add_note_reminder(&sample(&note_id, "same"))
        .expect("first");
    assert!(db.reminder_exists_by_intent_hash(&note_id, "same"));

    let second = db.add_note_reminder(&sample(&note_id, "same"));
    assert!(second.is_err(), "duplicate intent_hash must violate UNIQUE");

    assert!(!db.reminder_exists_by_intent_hash(&note_id, "other"));
}

#[test]
fn test_tombstone_excluded_from_exists() {
    let (db, _dir) = open_test_db();
    let note_id = make_note(&db);

    let created = db.add_note_reminder(&sample(&note_id, "h1")).expect("add");
    assert!(db.reminder_exists_by_intent_hash(&note_id, "h1"));

    db.set_reminder_state(&created.id, "tombstone")
        .expect("tombstone");
    assert!(
        !db.reminder_exists_by_intent_hash(&note_id, "h1"),
        "tombstoned reminder must not count as an active duplicate"
    );
    assert_eq!(db.reminders_for_note(&note_id).len(), 1);
}

#[test]
fn test_cascade_delete_note_removes_reminders() {
    let (db, _dir) = open_test_db();
    let note_id = make_note(&db);
    db.add_note_reminder(&sample(&note_id, "h1")).expect("add");

    db.delete_note(&note_id).expect("delete note");
    assert!(
        db.reminders_for_note(&note_id).is_empty(),
        "ON DELETE CASCADE must remove the reminder mapping"
    );
}

#[test]
fn test_new_note_reminder_from_intent() {
    let intent = ReminderIntent {
        action: "appeler Paul".into(),
        date: Some("2026-06-03".into()),
        time: Some("15:30".into()),
        time_end: None,
        recurrence: None,
        location: None,
    };
    let n = NewNoteReminder::from_intent(
        "note-1",
        "ek-9",
        "eventkit",
        &intent,
        Some("Europe/Brussels".into()),
    );
    assert_eq!(n.due_year, Some(2026));
    assert_eq!(n.due_month, Some(6));
    assert_eq!(n.due_day, Some(3));
    assert_eq!(n.due_hour, Some(15));
    assert_eq!(n.due_minute, Some(30));
    assert!(!n.is_all_day);
    assert!(!n.intent_hash.is_empty());
    assert!(!intent.is_event());

    let ranged = ReminderIntent {
        action: "réunion".into(),
        date: Some("2026-06-03".into()),
        time: Some("14:00".into()),
        time_end: Some("16:00".into()),
        recurrence: None,
        location: None,
    };
    assert!(ranged.is_event());
    assert!(ranged.intent_hash().contains("14:00-16:00"));
    assert_ne!(ranged.intent_hash(), intent.intent_hash());
}
