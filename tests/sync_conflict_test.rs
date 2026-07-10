use flowflow::domain::attachment::NewAttachment;
use flowflow::domain::folder::NewFolder;
use flowflow::domain::note::{NewTextNote, UpdateNote};
use flowflow::domain::thread::NewThread;
use flowflow::infrastructure::persistence::chunk_repo::ChunkRecord;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::conflict::{
    decide, dismiss_conflict, restore_note_conflict, ArchivedChunk,
    MergeOutcome, VersionInfo,
};
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
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let host_db = db_host.clone();
    let server = std::thread::spawn(move || {
        protocol::serve_sync_once(&host_db, &listener, 50, None)
    });
    protocol::sync_with_peer(db_client, host_device_id, "127.0.0.1", port, 50)
        .expect("client sync");
    server.join().expect("join").expect("server sync");
}

fn seed_chunks(db: &Database, note_id: &str, fill: f32) {
    let records: Vec<ChunkRecord> = (0..2)
        .map(|i| ChunkRecord {
            id: format!("note:{note_id}:{i}"),
            owner_id: note_id.to_string(),
            owner_kind: "note".to_string(),
            chunk_index: i,
            vector: vec![fill; 1536],
            content_hash: format!("hash-{fill}-{i}"),
            chunk_text: format!("chunk {fill} {i}"),
            title: "t".to_string(),
            tags: "[]".to_string(),
            created_at: "2026-06-09T00:00:00.000Z".to_string(),
        })
        .collect();
    db.replace_chunks(note_id, "note", &records)
        .expect("seed chunks");
    let conn = db.conn();
    flowflow::infrastructure::persistence::sync_meta::mark_entity_updated(
        &conn, "note", note_id,
    )
    .expect("bump owner");
}

fn first_vector_value(c: &ArchivedChunk) -> f32 {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    let blob = URL_SAFE_NO_PAD.decode(&c.vector_b64).expect("b64");
    f32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]])
}

// ---- decide(): the merge matrix, pure and exhaustive ----

fn vi<'a>(
    vv: &'a str,
    hlc: Option<&'a str>,
    device: &'a str,
) -> VersionInfo<'a> {
    VersionInfo {
        version_vector: vv,
        updated_hlc: hlc,
        origin_device: device,
    }
}

#[test]
fn decide_equal_and_local_dominance_skip() {
    let out = decide(
        &vi(r#"{"a":2,"b":1}"#, Some("10"), "a"),
        &vi(r#"{"a":2,"b":1}"#, Some("20"), "b"),
    );
    assert_eq!(out, MergeOutcome::AlreadyCurrent, "equal vv must skip");

    let out = decide(
        &vi(r#"{"a":3,"b":1}"#, Some("10"), "a"),
        &vi(r#"{"a":2,"b":1}"#, Some("20"), "b"),
    );
    assert_eq!(out, MergeOutcome::AlreadyCurrent, "local dominance skips");
}

#[test]
fn decide_remote_dominance_takes_remote() {
    let out = decide(
        &vi(r#"{"a":2}"#, Some("99"), "a"),
        &vi(r#"{"a":2,"b":4}"#, Some("10"), "b"),
    );
    assert_eq!(out, MergeOutcome::TakeRemote, "remote dominance applies");
}

#[test]
fn decide_concurrent_hlc_breaks_the_tie_both_ways() {
    let local = r#"{"a":2,"b":0}"#;
    let remote = r#"{"a":1,"b":3}"#;

    let out = decide(
        &vi(local, Some("2026-06-09T10:00:00Z"), "a"),
        &vi(remote, Some("2026-06-09T10:00:10Z"), "b"),
    );
    let MergeOutcome::Concurrent {
        remote_wins,
        joined,
        corrupt_local,
        corrupt_remote,
    } = out
    else {
        panic!("expected concurrent");
    };
    assert!(remote_wins, "+10s remote clock wins the tie");
    assert!(!corrupt_local && !corrupt_remote);
    assert_eq!(joined.get("a"), Some(&2));
    assert_eq!(joined.get("b"), Some(&3));

    let out = decide(
        &vi(local, Some("2026-06-09T10:00:10Z"), "a"),
        &vi(remote, Some("2026-06-09T10:00:00Z"), "b"),
    );
    assert!(
        matches!(
            out,
            MergeOutcome::Concurrent {
                remote_wins: false,
                ..
            }
        ),
        "+10s local clock keeps local"
    );
}

#[test]
fn decide_concurrent_equal_hlc_falls_back_to_device_id() {
    let local = r#"{"a":1,"b":0}"#;
    let remote = r#"{"a":0,"b":1}"#;
    let hlc = Some("2026-06-09T10:00:00Z");

    let out = decide(&vi(local, hlc, "aaaa"), &vi(remote, hlc, "zzzz"));
    assert!(
        matches!(
            out,
            MergeOutcome::Concurrent {
                remote_wins: true,
                ..
            }
        ),
        "higher device id wins an exact hlc tie"
    );
    let out = decide(&vi(local, hlc, "zzzz"), &vi(remote, hlc, "aaaa"));
    assert!(matches!(
        out,
        MergeOutcome::Concurrent {
            remote_wins: false,
            ..
        }
    ));
}

#[test]
fn decide_corrupt_vv_forces_concurrent_never_silent_loss() {
    let out = decide(
        &vi("THIS IS NOT JSON", Some("10"), "a"),
        &vi(r#"{"b":1}"#, Some("20"), "b"),
    );
    let MergeOutcome::Concurrent {
        corrupt_local,
        corrupt_remote,
        joined,
        ..
    } = out
    else {
        panic!("corrupt vv must be a forced conflict");
    };
    assert!(corrupt_local, "the unreadable side must be identified");
    assert!(!corrupt_remote);
    assert_eq!(joined.get("b"), Some(&1), "join keeps the readable side");
}

// ---- E2E: archive carries the losing vectors, children survive ----

#[test]
fn conflict_archives_losing_vectors_and_keeps_children_intact() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = db_a
        .create_text_note(&NewTextNote {
            title: Some("base".into()),
            content: "base content".into(),
            tags: vec![],
        })
        .expect("note")
        .id;
    db_a.create_attachment(&NewAttachment {
        note_id: note.clone(),
        filename: "doc.txt".into(),
        content_text: "attached text".into(),
    })
    .expect("attachment");
    db_a.add_audio(&note, "voice.wav", 2.5).expect("audio");
    let folder = db_a
        .create_folder(&NewFolder {
            name: "dossier".into(),
            description: None,
            parent_id: None,
        })
        .expect("folder")
        .id;
    db_a.add_note_to_folder(&note, &folder).expect("link");
    sync_once(&db_a, &db_b, &id_a);

    db_a.update_note(
        &note,
        &UpdateNote {
            title: Some("edited on A".into()),
            content: Some("content from A".into()),
            tags: None,
        },
    )
    .expect("update a");
    seed_chunks(&db_a, &note, 0.25);
    db_b.update_note(
        &note,
        &UpdateNote {
            title: Some("edited on B".into()),
            content: Some("content from B".into()),
            tags: None,
        },
    )
    .expect("update b");
    seed_chunks(&db_b, &note, 0.75);

    sync_once(&db_a, &db_b, &id_a);

    let on_a = db_a.get_note(&note).expect("q").expect("row");
    let on_b = db_b.get_note(&note).expect("q").expect("row");
    assert_eq!(on_a.title, on_b.title, "both sides converge");

    // C17: the surviving note keeps its id, so EVERY child stays attached.
    for db in [&db_a, &db_b] {
        assert_eq!(
            db.list_attachments_for_note(&note).expect("atts").len(),
            1,
            "attachment must survive the conflict"
        );
        assert_eq!(db.list_audios(&note).expect("audios").len(), 1);
        assert_eq!(db.folders_for_note(&note).expect("folders").len(), 1);
    }

    let conflicts: Vec<_> = db_a
        .list_unresolved_conflicts()
        .expect("list a")
        .into_iter()
        .chain(db_b.list_unresolved_conflicts().expect("list b"))
        .collect();
    assert_eq!(conflicts.len(), 1, "exactly one archived conflict");
    let conflict = &conflicts[0];

    let losing_fill = if on_a.title.as_deref() == Some("edited on A") {
        0.75
    } else {
        0.25
    };
    assert!(
        conflict
            .losing_snapshot_json
            .contains(if losing_fill == 0.75 {
                "content from B"
            } else {
                "content from A"
            }),
        "snapshot must hold the losing text"
    );
    let raw = conflict
        .losing_vector_ref
        .as_ref()
        .expect("losing vectors must be archived");
    let archived: Vec<ArchivedChunk> =
        serde_json::from_str(raw).expect("decodable vector ref");
    assert_eq!(archived.len(), 2, "both losing chunks archived");
    assert_eq!(first_vector_value(&archived[0]), losing_fill);
}

#[test]
fn restore_note_conflict_brings_back_text_and_vectors() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let note = db_a
        .create_text_note(&NewTextNote {
            title: Some("base".into()),
            content: "base".into(),
            tags: vec![],
        })
        .expect("note")
        .id;
    sync_once(&db_a, &db_b, &id_a);
    db_a.update_note(
        &note,
        &UpdateNote {
            title: Some("A version".into()),
            content: Some("body from A".into()),
            tags: None,
        },
    )
    .expect("update a");
    seed_chunks(&db_a, &note, 0.25);
    db_b.update_note(
        &note,
        &UpdateNote {
            title: Some("B version".into()),
            content: Some("body from B".into()),
            tags: None,
        },
    )
    .expect("update b");
    seed_chunks(&db_b, &note, 0.75);
    sync_once(&db_a, &db_b, &id_a);

    let (db_with, conflict) = if let Some(c) = db_a
        .list_unresolved_conflicts()
        .expect("list a")
        .into_iter()
        .next()
    {
        (&db_a, c)
    } else {
        (
            &db_b,
            db_b.list_unresolved_conflicts()
                .expect("list b")
                .into_iter()
                .next()
                .expect("one conflict somewhere"),
        )
    };

    let restored = restore_note_conflict(db_with, &conflict).expect("restore");
    assert_ne!(restored.id, note, "restored as a NEW note, no id fork");
    assert!(restored.content.starts_with("body from"));

    let chunks = db_with
        .chunks_for_owner(&restored.id, "note")
        .expect("chunks");
    assert_eq!(chunks.len(), 2, "archived vectors copied back");
    assert_eq!(chunks[0].id, format!("note:{}:0", restored.id));
    assert_eq!(chunks[0].vector.len(), 1536);

    assert_eq!(
        db_with.count_unresolved_conflicts().expect("count"),
        0,
        "restore resolves the conflict"
    );

    assert!(db_with.get_note(&restored.id).expect("q").is_some());
    let meta_exists: i64 = db_with
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sync_row_meta
             WHERE entity_kind = 'note' AND entity_id = ?1",
            [&restored.id],
            |r| r.get(0),
        )
        .expect("meta");
    assert_eq!(meta_exists, 1, "restored note is tracked and will sync");

    assert!(
        restore_note_conflict(db_with, &conflict).is_err(),
        "a second restore of the same conflict must be refused"
    );

    // The restored note AND its copied vectors must reach the other device
    // on the next session (the chunk copy bumps the owner meta).
    sync_once(&db_a, &db_b, &id_a);
    let db_other: &Database = if Arc::ptr_eq(db_with, &db_a) {
        &db_b
    } else {
        &db_a
    };
    let on_other = db_other
        .get_note(&restored.id)
        .expect("q")
        .expect("restored note propagates");
    assert!(on_other.content.starts_with("body from"));
    assert_eq!(
        db_other
            .chunks_for_owner(&restored.id, "note")
            .expect("chunks")
            .len(),
        2,
        "restored vectors propagate with their note"
    );
}

// RFC 0007: restoring a conflicted note must land it back in the surviving
// original's folder + thread, not detached.
fn seed_foldered_threaded_conflict(
    db_a: &Arc<Database>,
    db_b: &Arc<Database>,
    id_a: &str,
) -> (String, String, String) {
    let folder = db_a
        .create_folder(&NewFolder {
            name: "dossier".into(),
            description: None,
            parent_id: None,
        })
        .expect("folder")
        .id;
    let thread = db_a
        .create_thread(&NewThread {
            title: "fil".into(),
            folder_id: None,
        })
        .expect("thread")
        .id;
    let note = db_a
        .create_text_note_in_thread(
            &NewTextNote {
                title: Some("base".into()),
                content: "base".into(),
                tags: vec![],
            },
            &thread,
        )
        .expect("note")
        .id;
    db_a.create_text_note_in_thread(
        &NewTextNote {
            title: Some("sibling".into()),
            content: "sibling".into(),
            tags: vec![],
        },
        &thread,
    )
    .expect("sibling makes a real >=2 thread");
    db_a.add_note_to_folder(&note, &folder).expect("link");
    sync_once(db_a, db_b, id_a);

    db_a.update_note(
        &note,
        &UpdateNote {
            title: Some("A version".into()),
            content: Some("body from A".into()),
            tags: None,
        },
    )
    .expect("update a");
    db_b.update_note(
        &note,
        &UpdateNote {
            title: Some("B version".into()),
            content: Some("body from B".into()),
            tags: None,
        },
    )
    .expect("update b");
    sync_once(db_a, db_b, id_a);
    (folder, thread, note)
}

fn losing_side<'a>(
    db_a: &'a Arc<Database>,
    db_b: &'a Arc<Database>,
) -> (
    &'a Arc<Database>,
    flowflow::infrastructure::persistence::conflict_repo::SyncConflict,
) {
    if let Some(c) = db_a
        .list_unresolved_conflicts()
        .expect("list a")
        .into_iter()
        .next()
    {
        (db_a, c)
    } else {
        (
            db_b,
            db_b.list_unresolved_conflicts()
                .expect("list b")
                .into_iter()
                .next()
                .expect("one conflict somewhere"),
        )
    }
}

#[test]
fn restore_conflict_inherits_folder_and_thread() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let (folder, thread, note) =
        seed_foldered_threaded_conflict(&db_a, &db_b, &id_a);
    let (db_with, conflict) = losing_side(&db_a, &db_b);

    let restored = restore_note_conflict(db_with, &conflict).expect("restore");
    assert_ne!(restored.id, note, "restored as a NEW note");
    assert_eq!(
        restored.thread_id.as_deref(),
        Some(thread.as_str()),
        "restored note inherits the surviving original's thread"
    );
    let folders = db_with.folders_for_note(&restored.id).expect("folders");
    assert_eq!(folders.len(), 1, "restored note inherits the folder");
    assert_eq!(folders[0].id, folder);
}

// The user's literal scenario: a restored note's inherited thread + folder must
// survive the NEXT sync, not just exist locally. Proves RFC 0007 D5 (re-link +
// thread_id propagate on the fresh id) end to end, through apply_batch.
#[test]
fn restore_conflict_inherited_structure_propagates_to_peer() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let (folder, thread, _note) =
        seed_foldered_threaded_conflict(&db_a, &db_b, &id_a);
    let (db_with, conflict) = losing_side(&db_a, &db_b);

    let restored = restore_note_conflict(db_with, &conflict).expect("restore");
    sync_once(&db_a, &db_b, &id_a);

    let db_other: &Database = if Arc::ptr_eq(db_with, &db_a) {
        &db_b
    } else {
        &db_a
    };
    let on_other = db_other
        .get_note(&restored.id)
        .expect("q")
        .expect("restored note reaches the peer");
    assert_eq!(
        on_other.thread_id.as_deref(),
        Some(thread.as_str()),
        "inherited thread membership crosses the wire"
    );
    assert!(
        db_other.get_thread(&thread).expect("q").is_some(),
        "the thread row exists on the peer"
    );
    let folders = db_other.folders_for_note(&restored.id).expect("folders");
    assert!(
        folders.iter().any(|f| f.id == folder),
        "inherited folder link crosses the wire"
    );
}

// RFC 0007 F4: the thread_id guard drops a dangling reference. If the original
// note still carries a thread_id but the thread ROW is gone, the restored note
// must come back flat (no thread) while still inheriting the folder.
#[test]
fn restore_conflict_drops_dangling_thread_keeps_folder() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let (_folder, thread, note) =
        seed_foldered_threaded_conflict(&db_a, &db_b, &id_a);
    let (db_with, conflict) = losing_side(&db_a, &db_b);

    // Inject the dangling state the get_thread guard exists to catch: a note
    // still carrying thread_id while the thread row is gone. FK is ON locally
    // (ON DELETE SET NULL would null the member), so drop the row with FK off -
    // exactly how it arises on a peer, where apply runs foreign_keys=OFF.
    {
        let conn = db_with.conn();
        conn.execute_batch("PRAGMA foreign_keys=OFF;")
            .expect("fk off");
        conn.execute("DELETE FROM threads WHERE id = ?1", [&thread])
            .expect("raw delete thread row");
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .expect("fk on");
    }
    assert_eq!(
        db_with
            .get_note(&note)
            .expect("q")
            .expect("original alive")
            .thread_id
            .as_deref(),
        Some(thread.as_str()),
        "original still references the now-missing thread"
    );

    let restored = restore_note_conflict(db_with, &conflict).expect("restore");
    assert!(
        restored.thread_id.is_none(),
        "dangling thread reference is dropped, restored note is flat"
    );
    assert_eq!(
        db_with
            .folders_for_note(&restored.id)
            .expect("folders")
            .len(),
        1,
        "folder is still inherited even when the thread is gone"
    );
}

#[test]
fn restore_conflict_falls_back_to_flat_when_original_gone() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _) = pair(&db_a, &db_b);

    let (_folder, _thread, _note) =
        seed_foldered_threaded_conflict(&db_a, &db_b, &id_a);
    let (db_with, conflict) = losing_side(&db_a, &db_b);

    db_with
        .delete_note(&conflict.entity_id)
        .expect("delete the surviving original before restore");

    let restored = restore_note_conflict(db_with, &conflict).expect("restore");
    assert!(
        restored.thread_id.is_none(),
        "no thread inherited when the original is gone"
    );
    assert_eq!(
        db_with
            .folders_for_note(&restored.id)
            .expect("folders")
            .len(),
        0,
        "no folder inherited when the original is gone"
    );
}

#[test]
fn dismiss_marks_resolved_and_unknown_id_errors() {
    let (db, _d) = open_db();
    db.conn()
        .execute(
            "INSERT INTO sync_conflicts
                (entity_kind, entity_id, losing_vv, losing_snapshot_json,
                 losing_vector_ref, created_hlc, resolved)
             VALUES ('note','x','{}','{\"title\":\"t\"}',NULL,NULL,0)",
            [],
        )
        .expect("seed conflict");
    let listed = db.list_unresolved_conflicts().expect("list");
    assert_eq!(listed.len(), 1);

    dismiss_conflict(&db, listed[0].id).expect("dismiss");
    assert_eq!(db.count_unresolved_conflicts().expect("count"), 0);
    assert!(
        dismiss_conflict(&db, 9999).is_err(),
        "unknown conflict id must error"
    );
}
