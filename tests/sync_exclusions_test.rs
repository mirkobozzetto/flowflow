// RFC 0004 T23: sync perimeter exclusions + reminder merge by intent.
// Accept (C20): API keys and pending_transcriptions never cross the wire;
// a reminder synced between devices never collides on UNIQUE(note_id,
// intent_hash) and never double-notifies; a cancel kills the SAME intent on
// every device even when the rows have different ids (cross-id cancel); a
// later re-add of the intent wins over the cancel.

use flowflow::domain::note::NewTextNote;
use flowflow::domain::NewNoteReminder;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::{peers, protocol};
use std::net::TcpListener;
use std::sync::{Arc, Once};
use tempfile::tempdir;

static ADVERTISE: Once = Once::new();

fn open_db() -> (Arc<Database>, tempfile::TempDir) {
    let dir = tempdir().expect("db tempdir");
    let db =
        Database::open_at(dir.path().join("flowflow.db")).expect("open_at");
    (Arc::new(db), dir)
}

fn pair(db_a: &Arc<Database>, db_b: &Arc<Database>) -> (String, String) {
    ADVERTISE.call_once(|| {
        std::env::set_var("FLOWFLOW_SYNC_ADVERTISE_ADDR", "127.0.0.1");
    });
    let host = peers::start_pairing_host(db_a.clone()).expect("host");
    let mut payload =
        peers::decode_pairing_uri(&host.uri).expect("decode host uri");
    payload.addr = "127.0.0.1".to_string();
    let uri = peers::encode_pairing_uri(&payload).expect("re-encode uri");
    let id_a = peers::join_pairing(db_b, &uri).expect("join pairing");
    for _ in 0..100 {
        if host.status() != peers::PairingStatus::Waiting {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let id_b = peers::ensure_sync_identity(db_b).expect("id b").device_id;
    assert!(matches!(host.status(), peers::PairingStatus::Paired { .. }));
    (id_a, id_b)
}

fn sync_once(
    db_host: &Arc<Database>,
    db_client: &Arc<Database>,
    host_device_id: &str,
) -> (protocol::SyncStats, protocol::SyncStats) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_host.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, 50, None)
    });
    let client_stats = protocol::sync_with_peer(
        db_client,
        host_device_id,
        "127.0.0.1",
        port,
        50,
    )
    .expect("client sync");
    let host_stats = server.join().expect("join").expect("server sync").stats;
    (host_stats, client_stats)
}

fn make_note(db: &Database, title: &str) -> String {
    db.create_text_note(&NewTextNote {
        title: Some(title.to_string()),
        content: format!("{title} body"),
        tags: vec![],
    })
    .expect("create note")
    .id
}

fn add_reminder(db: &Database, note_id: &str, intent_hash: &str) -> String {
    db.add_note_reminder(&NewNoteReminder {
        note_id: note_id.to_string(),
        reminder_id: uuid::Uuid::new_v4().to_string(),
        backend: "usernotifications".to_string(),
        intent_hash: intent_hash.to_string(),
        due_year: Some(2026),
        due_month: Some(7),
        due_day: Some(1),
        due_hour: Some(9),
        due_minute: Some(0),
        is_all_day: false,
        tz_id: Some("Europe/Brussels".to_string()),
        recurrence: None,
    })
    .expect("add reminder")
    .id
}

fn reminder_states(db: &Database, note_id: &str) -> Vec<(String, String)> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(
            "SELECT id, state FROM note_reminders WHERE note_id = ?1
             ORDER BY id",
        )
        .expect("prepare");
    stmt.query_map([note_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("query")
        .flatten()
        .collect()
}

fn active_count(db: &Database, note_id: &str) -> usize {
    reminder_states(db, note_id)
        .iter()
        .filter(|(_, s)| s == "active")
        .count()
}

#[test]
fn api_keys_and_pending_transcriptions_never_cross() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    db_a.set_setting("openai_api_key", "sk-super-secret")
        .expect("set key");
    db_a.set_setting("anthropic_api_key", "sk-ant-secret")
        .expect("set key");
    db_a.set_setting("soniox_api_key", "sx-secret")
        .expect("set key");
    let note = make_note(&db_a, "carrier");
    db_a.conn()
        .execute(
            "INSERT INTO pending_transcriptions
                (note_id, transcription_id, soniox_file_id)
             VALUES (?1, 'job-123', 'file-456')",
            [&note],
        )
        .expect("pending job");

    sync_once(&db_a, &db_b, &id_a);

    assert!(
        db_b.get_note(&note).expect("q").is_some(),
        "data still flows"
    );
    for key in ["openai_api_key", "anthropic_api_key", "soniox_api_key"] {
        assert_eq!(
            db_b.get_setting(key),
            None,
            "{key} must never reach another device"
        );
    }
    let pending_on_b: i64 = db_b
        .conn()
        .query_row("SELECT COUNT(*) FROM pending_transcriptions", [], |r| {
            r.get(0)
        })
        .expect("count");
    assert_eq!(pending_on_b, 0, "job state is device-local");
    // Structural guarantee: the tracking layer itself never records those
    // tables, so nothing can ever collect them.
    let tracked: i64 = db_a
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sync_row_meta
             WHERE entity_kind IN ('settings', 'pending_transcription',
                                   'pending_transcriptions')",
            [],
            |r| r.get(0),
        )
        .expect("meta scan");
    assert_eq!(tracked, 0);
}

#[test]
fn cross_id_cancel_kills_the_twin_intent_everywhere() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "remind");
    sync_once(&db_a, &db_b, &id_a);

    // The same intent created independently on both devices: different row
    // ids, same (note_id, intent_hash).
    let rid_a = add_reminder(&db_a, &note, "intent-x");
    let rid_b = add_reminder(&db_b, &note, "intent-x");
    assert_ne!(rid_a, rid_b);
    sync_once(&db_a, &db_b, &id_a);
    assert_eq!(active_count(&db_a, &note), 1, "one active intent on A");
    assert_eq!(active_count(&db_b, &note), 1, "one active intent on B");

    // Cancel on A. The tombstone travels under A's row id; B must kill its
    // OWN twin row anyway (cross-id cancel) or it double-notifies forever.
    db_a.set_reminder_state(&rid_a, "tombstone")
        .expect("cancel");
    sync_once(&db_a, &db_b, &id_a);
    sync_once(&db_a, &db_b, &id_a);
    assert_eq!(active_count(&db_a, &note), 0, "cancelled on A");
    assert_eq!(active_count(&db_b, &note), 0, "twin cancelled on B");

    let (h, c) = sync_once(&db_a, &db_b, &id_a);
    assert_eq!(h.conflicts + c.conflicts, 0, "no conflict churn");
}

#[test]
fn re_adding_the_intent_after_a_cancel_reactivates_it_everywhere() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "remind again");
    sync_once(&db_a, &db_b, &id_a);
    add_reminder(&db_a, &note, "intent-y");
    add_reminder(&db_b, &note, "intent-y");
    sync_once(&db_a, &db_b, &id_a);

    db_a.set_reminder_state(
        &reminder_states(&db_a, &note)[0].0.clone(),
        "tombstone",
    )
    .expect("cancel");
    sync_once(&db_a, &db_b, &id_a);
    sync_once(&db_a, &db_b, &id_a);
    assert_eq!(active_count(&db_b, &note), 0, "cancel propagated");

    // The user changes their mind LATER on B: the newer intent wins over the
    // older cancel, on both devices.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let readded = add_reminder(&db_b, &note, "intent-y");
    let states_b = reminder_states(&db_b, &note);
    assert_eq!(
        states_b.len(),
        1,
        "re-add must reactivate the tombstoned row, not collide on UNIQUE"
    );
    assert_eq!(states_b[0].0, readded);

    sync_once(&db_a, &db_b, &id_a);
    sync_once(&db_a, &db_b, &id_a);
    assert_eq!(active_count(&db_b, &note), 1, "active again on B");
    assert_eq!(active_count(&db_a, &note), 1, "twin reactivated on A");

    let (h, c) = sync_once(&db_a, &db_b, &id_a);
    assert_eq!(h.conflicts + c.conflicts, 0, "stable, no ping-pong");
}
