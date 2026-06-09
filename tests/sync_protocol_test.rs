use flowflow::db::chunk_repo::ChunkRecord;
use flowflow::db::Database;
use flowflow::models::note::{NewTextNote, UpdateNote};
use flowflow::services::sync::{peers, protocol};
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
    batch_rows: usize,
) -> (protocol::SyncStats, protocol::SyncStats) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_host.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, batch_rows, None)
    });
    let client_stats = protocol::sync_with_peer(
        db_client,
        host_device_id,
        "127.0.0.1",
        port,
        batch_rows,
    )
    .expect("client sync");
    let host_stats = server.join().expect("join").expect("server sync");
    (host_stats, client_stats)
}

fn make_note(db: &Database, title: &str, content: &str) -> String {
    db.create_text_note(&NewTextNote {
        title: Some(title.to_string()),
        content: content.to_string(),
        tags: vec!["sync".to_string()],
    })
    .expect("create note")
    .id
}

#[test]
fn notes_flow_both_directions_and_resync_is_idempotent() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let note_a = make_note(&db_a, "from A", "alpha content");
    let note_b = make_note(&db_b, "from B", "beta content");

    let (host_stats, client_stats) = sync_once(&db_a, &db_b, &id_a, 50);
    assert!(host_stats.pushed >= 1);
    assert!(client_stats.pushed >= 1);

    let a_on_b = db_b.get_note(&note_a).expect("query").expect("row");
    assert_eq!(a_on_b.title.as_deref(), Some("from A"));
    assert_eq!(a_on_b.content, "alpha content");
    let b_on_a = db_a.get_note(&note_b).expect("query").expect("row");
    assert_eq!(b_on_a.content, "beta content");

    let (host2, client2) = sync_once(&db_a, &db_b, &id_a, 50);
    assert_eq!(host2.pushed, 0, "second session must push nothing");
    assert_eq!(client2.pushed, 0, "second session must push nothing");
    assert_eq!(host2.applied, 0);
    assert_eq!(client2.applied, 0);
}

#[test]
fn edit_propagates_and_meta_converges() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "v1", "first body");
    sync_once(&db_a, &db_b, &id_a, 50);

    db_a.update_note(
        &note,
        &UpdateNote {
            title: Some("v2".to_string()),
            content: Some("second body".to_string()),
            tags: None,
        },
    )
    .expect("update");
    sync_once(&db_a, &db_b, &id_a, 50);

    let on_b = db_b.get_note(&note).expect("query").expect("row");
    assert_eq!(on_b.title.as_deref(), Some("v2"));
    assert_eq!(on_b.content, "second body");

    let meta_a = read_meta(&db_a, "note", &note);
    let meta_b = read_meta(&db_b, "note", &note);
    assert_eq!(meta_a, meta_b, "meta must be identical after sync");
}

fn read_meta(
    db: &Database,
    kind: &str,
    id: &str,
) -> (String, String, i64, i64) {
    db.conn()
        .query_row(
            "SELECT version_vector, origin_device, origin_seq, deleted
             FROM sync_row_meta WHERE entity_kind = ?1 AND entity_id = ?2",
            rusqlite::params![kind, id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("meta row")
}

#[test]
fn delete_propagates_with_children_and_chunks() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "doomed", "note with attachment body");
    let att = db_a
        .create_attachment(&flowflow::models::attachment::NewAttachment {
            note_id: note.clone(),
            filename: "doc.txt".to_string(),
            content_text: "attached words".to_string(),
        })
        .expect("attachment")
        .id;
    seed_chunks(&db_a, &note, "note");

    sync_once(&db_a, &db_b, &id_a, 50);
    assert!(db_b.get_note(&note).expect("q").is_some());
    assert!(db_b.get_attachment(&att).expect("q").is_some());
    assert_eq!(db_b.count_chunks_for_owner(&note, "note").expect("c"), 2);

    db_a.delete_note(&note).expect("delete");
    sync_once(&db_a, &db_b, &id_a, 50);

    assert!(db_b.get_note(&note).expect("q").is_none());
    assert!(db_b.get_attachment(&att).expect("q").is_none());
    assert_eq!(db_b.count_chunks_for_owner(&note, "note").expect("c"), 0);
    let (_, _, _, deleted) = read_meta(&db_b, "note", &note);
    assert_eq!(deleted, 1, "tombstone meta must be applied");
}

fn seed_chunks(db: &Database, owner_id: &str, owner_kind: &str) {
    let records: Vec<ChunkRecord> = (0..2)
        .map(|i| ChunkRecord {
            id: format!("note:{owner_id}:{i}"),
            owner_id: owner_id.to_string(),
            owner_kind: owner_kind.to_string(),
            chunk_index: i,
            vector: vec![0.5; 1536],
            content_hash: format!("hash-{i}"),
            chunk_text: format!("chunk text {i}"),
            title: "t".to_string(),
            tags: "[]".to_string(),
            created_at: "2026-06-09T00:00:00.000Z".to_string(),
        })
        .collect();
    db.replace_chunks(owner_id, owner_kind, &records)
        .expect("seed chunks");
    let conn = db.conn();
    flowflow::db::sync_meta::mark_entity_updated(&conn, owner_kind, owner_id)
        .expect("bump owner");
}

#[test]
fn chunk_blobs_travel_with_their_note() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "vectors", "a note that has embeddings");
    seed_chunks(&db_a, &note, "note");

    sync_once(&db_a, &db_b, &id_a, 50);

    let on_b = db_b.chunks_for_owner(&note, "note").expect("chunks");
    assert_eq!(on_b.len(), 2);
    assert_eq!(on_b[0].vector.len(), 1536);
    assert_eq!(on_b[0].vector[0], 0.5);
    assert_eq!(on_b[1].chunk_text, "chunk text 1");
}

#[test]
fn concurrent_edits_converge_and_archive_one_conflict() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "base", "shared base");
    sync_once(&db_a, &db_b, &id_a, 50);

    db_a.update_note(
        &note,
        &UpdateNote {
            title: Some("edited on A".to_string()),
            content: None,
            tags: None,
        },
    )
    .expect("update a");
    db_b.update_note(
        &note,
        &UpdateNote {
            title: Some("edited on B".to_string()),
            content: None,
            tags: None,
        },
    )
    .expect("update b");

    sync_once(&db_a, &db_b, &id_a, 50);

    let on_a = db_a.get_note(&note).expect("q").expect("row");
    let on_b = db_b.get_note(&note).expect("q").expect("row");
    assert_eq!(on_a.title, on_b.title, "both sides must converge");
    assert!(
        on_a.title.as_deref() == Some("edited on A")
            || on_a.title.as_deref() == Some("edited on B")
    );

    let conflicts_a = count_conflicts(&db_a, &note);
    let conflicts_b = count_conflicts(&db_b, &note);
    assert_eq!(
        conflicts_a + conflicts_b,
        1,
        "exactly one archived conflict total"
    );
    let loser_expected = if on_a.title.as_deref() == Some("edited on A") {
        "edited on B"
    } else {
        "edited on A"
    };
    let read_snapshot = |db: &Database| -> Option<String> {
        db.conn()
            .query_row(
                "SELECT losing_snapshot_json FROM sync_conflicts
                 WHERE entity_id = ?1",
                [&note],
                |r| r.get(0),
            )
            .ok()
    };
    let snapshot = read_snapshot(&db_a)
        .or_else(|| read_snapshot(&db_b))
        .expect("conflict snapshot");
    assert!(
        snapshot.contains(loser_expected),
        "losing version must be archived, got: {snapshot}"
    );

    let (h2, c2) = sync_once(&db_a, &db_b, &id_a, 50);
    assert_eq!(h2.conflicts + c2.conflicts, 0, "no repeated conflicts");
    let meta_a = read_meta(&db_a, "note", &note);
    let meta_b = read_meta(&db_b, "note", &note);
    assert_eq!(meta_a.0, meta_b.0, "version vectors must converge");
}

fn count_conflicts(db: &Database, entity_id: &str) -> i64 {
    db.conn()
        .query_row(
            "SELECT COUNT(*) FROM sync_conflicts WHERE entity_id = ?1",
            [entity_id],
            |r| r.get(0),
        )
        .expect("count conflicts")
}

#[test]
fn cut_mid_push_resumes_without_loss_or_duplicates() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let mut ids = Vec::new();
    for i in 0..5 {
        ids.push(make_note(&db_b, &format!("n{i}"), &format!("body {i}")));
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_a.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, 2, Some(1))
    });
    let res = protocol::sync_with_peer(&db_b, &id_a, "127.0.0.1", port, 2);
    assert!(res.is_err(), "cut session must surface an error");
    assert!(server.join().expect("join").is_err());

    let after_cut = db_a.list_notes().expect("list").len();
    assert!(
        after_cut < 5,
        "cut must have stopped mid-transfer (got {after_cut})"
    );

    let (host2, _client2) = sync_once(&db_a, &db_b, &id_a, 2);
    let notes_on_a = db_a.list_notes().expect("list");
    assert_eq!(notes_on_a.len(), 5, "resume must deliver the rest");
    for id in &ids {
        assert!(db_a.get_note(id).expect("q").is_some());
    }
    assert!(
        host2.applied < 5 + host2.skipped,
        "already-applied rows must not duplicate"
    );

    let (host3, client3) = sync_once(&db_a, &db_b, &id_a, 2);
    assert_eq!(host3.pushed + client3.pushed, 0, "fully converged");
}

#[test]
fn same_reminder_intent_on_both_sides_does_not_crash() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "remind", "reminder body");
    sync_once(&db_a, &db_b, &id_a, 50);

    add_reminder(&db_a, &note, "intent-1");
    add_reminder(&db_b, &note, "intent-1");

    let (h, c) = sync_once(&db_a, &db_b, &id_a, 50);
    assert!(h.pushed >= 1 && c.pushed >= 1);

    let count_a: i64 = db_a
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM note_reminders WHERE note_id = ?1",
            [&note],
            |r| r.get(0),
        )
        .expect("count");
    let count_b: i64 = db_b
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM note_reminders WHERE note_id = ?1",
            [&note],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(count_a, 1, "no duplicate intent on A");
    assert_eq!(count_b, 1, "no duplicate intent on B");
}

fn add_reminder(db: &Database, note_id: &str, intent_hash: &str) {
    db.conn()
        .execute(
            "INSERT INTO note_reminders
                (id, note_id, reminder_id, backend, intent_hash, state)
             VALUES (?1, ?2, ?3, 'usernotifications', ?4, 'active')",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                note_id,
                uuid::Uuid::new_v4().to_string(),
                intent_hash,
            ],
        )
        .expect("insert reminder");
}

#[test]
fn reminder_delete_propagates_as_state_tombstone() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "remind", "note with a reminder");
    sync_once(&db_a, &db_b, &id_a, 50);

    add_reminder(&db_a, &note, "intent-z");
    sync_once(&db_a, &db_b, &id_a, 50);
    let reminder_id: String = db_b
        .conn()
        .query_row(
            "SELECT id FROM note_reminders WHERE note_id = ?1",
            [&note],
            |r| r.get(0),
        )
        .expect("reminder landed on B");

    let to_delete: String = db_a
        .conn()
        .query_row(
            "SELECT id FROM note_reminders WHERE note_id = ?1",
            [&note],
            |r| r.get(0),
        )
        .expect("reminder on A");
    db_a.delete_note_reminder(&to_delete).expect("delete");
    sync_once(&db_a, &db_b, &id_a, 50);

    let state: String = db_b
        .conn()
        .query_row(
            "SELECT state FROM note_reminders WHERE id = ?1",
            [&reminder_id],
            |r| r.get(0),
        )
        .expect("row kept on B (note still alive)");
    assert_eq!(state, "tombstone", "remote delete must flip state");
}

#[test]
fn corrupt_local_vv_forces_archived_conflict_not_silent_overwrite() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "fragile", "base body");
    sync_once(&db_a, &db_b, &id_a, 50);

    db_b.conn()
        .execute(
            "UPDATE sync_row_meta SET version_vector = 'not json'
             WHERE entity_kind = 'note' AND entity_id = ?1",
            [&note],
        )
        .expect("corrupt vv on B");

    db_a.update_note(
        &note,
        &UpdateNote {
            title: Some("edited after corruption".to_string()),
            content: None,
            tags: None,
        },
    )
    .expect("update");
    sync_once(&db_a, &db_b, &id_a, 50);

    assert_eq!(
        count_conflicts(&db_b, &note),
        1,
        "corrupt vv must archive, never silently resolve"
    );
    let on_a = db_a.get_note(&note).expect("q").expect("row");
    let on_b = db_b.get_note(&note).expect("q").expect("row");
    assert_eq!(on_a.title, on_b.title, "both sides converge");

    let (h2, c2) = sync_once(&db_a, &db_b, &id_a, 50);
    assert_eq!(
        h2.conflicts + c2.conflicts,
        0,
        "conflict must not repeat once re-authored"
    );
}

#[test]
fn unpaired_peer_is_refused() {
    let (db_a, _da) = open_db();
    let (db_c, _dc) = open_db();
    let id_a = peers::ensure_sync_identity(&db_a).expect("id").device_id;

    let res = protocol::sync_with_peer(&db_c, &id_a, "127.0.0.1", 1, 50);
    assert!(
        res.is_err(),
        "initiator without pairing must refuse locally"
    );

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_a.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, 50, None)
    });
    let mut raw =
        std::net::TcpStream::connect(("127.0.0.1", port)).expect("tcp connect");
    let hint = serde_json::json!({
        "kind": "sync_hint",
        "device_id": "intruder-device"
    })
    .to_string();
    use std::io::Write as _;
    let len = (hint.len() as u16).to_be_bytes();
    raw.write_all(&len).expect("len");
    raw.write_all(hint.as_bytes()).expect("hint");
    raw.flush().expect("flush");

    assert!(
        server.join().expect("join").is_err(),
        "host must refuse an unknown device_id"
    );
    assert!(db_a.list_notes().expect("list").is_empty());
}

#[test]
fn folders_links_and_conversations_sync() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let folder = db_a
        .create_folder(&flowflow::models::folder::NewFolder {
            name: "Inbox".to_string(),
            description: None,
            parent_id: None,
        })
        .expect("folder")
        .id;
    let note = make_note(&db_a, "filed", "folder note body");
    db_a.add_note_to_folder(&note, &folder).expect("link");

    let conv = db_a
        .create_conversation("hello sync")
        .expect("conversation");
    db_a.add_message(&conv.id, "user", "what is in my notes?", None)
        .expect("message");

    sync_once(&db_a, &db_b, &id_a, 50);

    assert!(db_b.get_folder(&folder).expect("q").is_some());
    let in_folder = db_b.list_notes_in_folder(&folder).expect("list");
    assert_eq!(in_folder.len(), 1);
    assert_eq!(in_folder[0].id, note);
    let msgs = db_b.list_messages(&conv.id).expect("messages");
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, "what is in my notes?");
}
