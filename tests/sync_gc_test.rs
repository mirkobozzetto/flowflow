// RFC 0004 T19: tombstone GC, add-wins-over-delete, full-state reconcile.
// Accept (C18): a deleted note (with children) stays dead through 3 syncs and
// through GC; a concurrent child add resurrects the parent on both sides; a
// device restored from an older backup converges through a full-state
// session with ZERO resurrection and ZERO data loss.

use flowflow::domain::attachment::NewAttachment;
use flowflow::domain::note::NewTextNote;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::{gc, peers, protocol};
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

fn make_note(db: &Database, title: &str, content: &str) -> String {
    db.create_text_note(&NewTextNote {
        title: Some(title.to_string()),
        content: content.to_string(),
        tags: vec![],
    })
    .expect("create note")
    .id
}

fn meta_exists(db: &Database, kind: &str, id: &str) -> bool {
    db.conn()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sync_row_meta
             WHERE entity_kind = ?1 AND entity_id = ?2)",
            rusqlite::params![kind, id],
            |r| r.get(0),
        )
        .expect("meta exists query")
}

fn max_gc_horizon(db: &Database) -> i64 {
    db.conn()
        .query_row(
            "SELECT COALESCE(MAX(gc_horizon), 0) FROM sync_peers",
            [],
            |r| r.get(0),
        )
        .expect("horizon query")
}

#[test]
fn deleted_note_stays_dead_through_gc_and_three_syncs() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "doomed", "note that will be deleted");
    let att = db_a
        .create_attachment(&NewAttachment {
            note_id: note.clone(),
            filename: "doc.txt".to_string(),
            content_text: "attached".to_string(),
        })
        .expect("attachment")
        .id;
    sync_once(&db_a, &db_b, &id_a);
    assert!(db_b.get_note(&note).expect("q").is_some());

    db_a.delete_note(&note).expect("delete");
    // GC before the peer acked the tombstone must purge NOTHING.
    let early = gc::run_gc(&db_a).expect("gc");
    assert!(
        meta_exists(&db_a, "note", &note),
        "tombstone purged before the peer acked it (purged {})",
        early.purged
    );

    sync_once(&db_a, &db_b, &id_a);
    assert!(db_b.get_note(&note).expect("q").is_none());
    assert!(db_b.get_attachment(&att).expect("q").is_none());

    let report = gc::run_gc(&db_a).expect("gc");
    assert!(report.purged >= 2, "note + child tombstones purged");
    assert!(!meta_exists(&db_a, "note", &note));
    assert!(!meta_exists(&db_a, "attachment", &att));
    assert!(max_gc_horizon(&db_a) > 0, "horizon must advance");

    for _ in 0..3 {
        let (h, c) = sync_once(&db_a, &db_b, &id_a);
        assert_eq!(h.conflicts + c.conflicts, 0, "no conflict churn");
        assert!(db_a.get_note(&note).expect("q").is_none());
        assert!(db_b.get_note(&note).expect("q").is_none());
        assert!(db_a.get_attachment(&att).expect("q").is_none());
        assert!(db_b.get_attachment(&att).expect("q").is_none());
    }
}

#[test]
fn gc_without_peers_purges_nothing() {
    let (db, _d) = open_db();
    let note = make_note(&db, "solo", "single-device note");
    db.delete_note(&note).expect("delete");
    let report = gc::run_gc(&db).expect("gc");
    assert_eq!(report.purged, 0, "no peers = nothing to reconcile = no GC");
    assert!(meta_exists(&db, "note", &note));
}

#[test]
fn concurrent_child_add_resurrects_the_parent_on_both_sides() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let note = make_note(&db_a, "parent", "note with diverging children");
    let att_old = db_a
        .create_attachment(&NewAttachment {
            note_id: note.clone(),
            filename: "old.txt".to_string(),
            content_text: "known to the deleter".to_string(),
        })
        .expect("old attachment")
        .id;
    sync_once(&db_a, &db_b, &id_a);

    // Diverge: A deletes the note (cascades old child), B adds a NEW child.
    db_a.delete_note(&note).expect("delete on A");
    let att_new = db_b
        .create_attachment(&NewAttachment {
            note_id: note.clone(),
            filename: "new.txt".to_string(),
            content_text: "added concurrently with the delete".to_string(),
        })
        .expect("new attachment")
        .id;

    // Session 1: B's add reaches A; A's tombstones reach B, where the
    // concurrent add VETOES them (parent re-authored alive on B).
    sync_once(&db_a, &db_b, &id_a);
    let on_b = db_b.get_note(&note).expect("q");
    assert!(on_b.is_some(), "add-wins must keep the parent alive on B");
    assert!(db_b.get_attachment(&att_new).expect("q").is_some());
    assert!(
        db_b.get_attachment(&att_old).expect("q").is_none(),
        "the child the deleter knew stays deleted"
    );

    // Session 2: B's resurrection (fresh seq) dominates A's tombstone.
    sync_once(&db_a, &db_b, &id_a);
    let on_a = db_a.get_note(&note).expect("q");
    assert!(on_a.is_some(), "resurrection must propagate back to A");
    assert_eq!(
        on_a.expect("row").content,
        "note with diverging children",
        "parent comes back with its content"
    );
    assert!(db_a.get_attachment(&att_new).expect("q").is_some());
    assert!(db_a.get_attachment(&att_old).expect("q").is_none());

    let (h3, c3) = sync_once(&db_a, &db_b, &id_a);
    assert_eq!(h3.pushed + c3.pushed, 0, "converged, no ping-pong");
}

fn checkpoint_and_copy(
    db: &Database,
    from: &std::path::Path,
    to: &std::path::Path,
) {
    db.checkpoint_wal().expect("checkpoint");
    std::fs::copy(from, to).expect("backup copy");
}

#[test]
fn restored_device_converges_full_state_without_resurrection() {
    let (db_a, _da) = open_db();
    let (db_b, dir_b) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);
    let path_b = dir_b.path().join("flowflow.db");
    let backup_b = dir_b.path().join("backup.db");

    // N1 exists on both, then B is backed up.
    let n1 = make_note(&db_b, "n1", "created before the backup");
    sync_once(&db_a, &db_b, &id_a);
    checkpoint_and_copy(&db_b, &path_b, &backup_b);

    // Life goes on: B creates N2 (synced), A deletes N1, the tombstone is
    // acked by B and then GC'd on A.
    let n2 = make_note(&db_b, "n2", "created after the backup");
    sync_once(&db_a, &db_b, &id_a);
    db_a.delete_note(&n1).expect("delete n1");
    sync_once(&db_a, &db_b, &id_a);
    assert!(db_b.get_note(&n1).expect("q").is_none());
    let report = gc::run_gc(&db_a).expect("gc");
    assert!(report.purged >= 1, "n1 tombstone must be GC'd on A");
    assert!(!meta_exists(&db_a, "note", &n1));

    // Disaster: B is restored from the old backup. N1 is alive again there,
    // N2 is lost, its seq counter and watermarks rolled back.
    db_b.checkpoint_wal().expect("checkpoint before restore");
    drop(db_b);
    let _ = std::fs::remove_file(dir_b.path().join("flowflow.db-wal"));
    let _ = std::fs::remove_file(dir_b.path().join("flowflow.db-shm"));
    std::fs::copy(&backup_b, &path_b).expect("restore copy");
    let db_b = Arc::new(Database::open_at(path_b).expect("reopen restored"));
    assert!(db_b.get_note(&n1).expect("q").is_some(), "restore sanity");
    assert!(db_b.get_note(&n2).expect("q").is_none(), "restore sanity");

    // Full-state session: zero resurrection (N1 stays dead everywhere, its
    // content archived as a conflict on A) and zero loss (N2 returns to B).
    sync_once(&db_a, &db_b, &id_a);
    assert!(
        db_a.get_note(&n1).expect("q").is_none(),
        "no resurrection on A"
    );
    assert!(
        db_b.get_note(&n1).expect("q").is_none(),
        "no resurrection on B"
    );
    assert!(
        db_b.get_note(&n2).expect("q").is_some(),
        "lost data restored"
    );
    let archived: i64 = db_a
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sync_conflicts WHERE entity_id = ?1",
            [&n1],
            |r| r.get(0),
        )
        .expect("conflict count");
    assert_eq!(archived, 1, "the re-deleted content must be archived");

    // The restored device keeps working: its reseeded counter puts new
    // writes above the peer watermark.
    let n3 = make_note(&db_b, "n3", "written after the restore");
    sync_once(&db_a, &db_b, &id_a);
    assert!(
        db_a.get_note(&n3).expect("q").is_some(),
        "post-restore writes must reach the peer"
    );
    let (h, c) = sync_once(&db_a, &db_b, &id_a);
    assert_eq!(h.pushed + c.pushed, 0, "fully converged after recovery");
}
