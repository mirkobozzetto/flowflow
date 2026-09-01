use flowflow::application::backup;
use flowflow::application::constants::EMBEDDING_DIMS;
use flowflow::domain::note::NewTextNote;
use flowflow::domain::UpdateNote;
use flowflow::infrastructure::persistence::chunk_repo::ChunkRecord;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::reconcile::{
    backfill_legacy_chunks, reconcile_once,
};
use flowflow::infrastructure::sync::{gc, peers, protocol};
use flowflow::infrastructure::vectordb::VectorStore;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};
use tempfile::tempdir;

static ADVERTISE: Once = Once::new();
static VDB_ENV_LOCK: Mutex<()> = Mutex::new(());

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
    let client_res = protocol::sync_with_peer(
        db_client,
        host_device_id,
        "127.0.0.1",
        port,
        50,
    );
    let server_res = server.join().expect("join");
    let client_stats = match client_res {
        Ok(s) => s,
        Err(e) => panic!(
            "client sync: {e:?} (server err: {:?})",
            server_res.as_ref().err()
        ),
    };
    let host_stats = server_res.expect("server sync").stats;
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

struct RestoreDirs {
    archive: PathBuf,
    paths: backup::RestorePaths,
    staging: PathBuf,
}

fn export_archive_of(db_dir: &Path, work: &Path) -> RestoreDirs {
    let db_file = db_dir.join("flowflow.db");
    let export_staging = work.join("export_staging");
    let snapshot = backup::create_scrubbed_snapshot(&db_file, &export_staging)
        .expect("snapshot");
    let archive = work.join("backup.ffbak.zip");
    let audio_dir = db_dir.join("audio");
    std::fs::create_dir_all(&audio_dir).expect("audio dir");
    backup::build_archive(&snapshot, &audio_dir, &archive).expect("archive");
    RestoreDirs {
        archive,
        paths: backup::RestorePaths {
            pending: work.join("pending_restore"),
            bak: work.join("restore_bak"),
            data_db: db_file,
            audio_dir,
            vectordb_dir: db_dir.join("vectordb"),
            error_file: work.join("restore_error.txt"),
        },
        staging: work.join("import_staging"),
    }
}

fn restore_device(db: Arc<Database>, dirs: &RestoreDirs) -> Arc<Database> {
    let validated = backup::validate_archive_at(
        &dirs.archive,
        &dirs.staging,
        &dirs.paths.pending,
        None,
    )
    .expect("validate");
    backup::stage_import_at(&validated, &dirs.paths.pending).expect("stage");
    drop(db);
    let outcome = backup::apply_pending_restore_at(&dirs.paths).expect("swap");
    assert_eq!(outcome, backup::RestoreOutcome::Committed);
    let restored =
        Database::open_at(dirs.paths.data_db.clone()).expect("reopen");
    assert_eq!(
        restored.get_setting("sync_restored_pending").as_deref(),
        Some("true"),
        "restored device must self-declare"
    );
    Arc::new(restored)
}

fn repair_with_rebind(
    db_intact: &Arc<Database>,
    db_restored: &Arc<Database>,
    restored_device_id: &str,
) {
    let host = peers::start_pairing_host(db_intact.clone()).expect("host");
    let mut payload =
        peers::decode_pairing_uri(&host.uri).expect("decode host uri");
    payload.addr = "127.0.0.1".to_string();
    let uri = peers::encode_pairing_uri(&payload).expect("re-encode uri");

    let first = peers::join_pairing(db_restored, &uri);
    assert!(
        first.is_err(),
        "rebind without confirmation must be refused"
    );
    for _ in 0..100 {
        if matches!(host.status(), peers::PairingStatus::Failed(_)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    match host.status() {
        peers::PairingStatus::Failed(msg) => assert!(
            msg.contains(peers::REBIND_REFUSED_MARKER),
            "unexpected refusal: {msg}"
        ),
        other => panic!("expected refusal, got {other:?}"),
    }

    peers::authorize_rebind(db_intact, restored_device_id)
        .expect("authorize rebind");
    peers::join_pairing(db_restored, &uri).expect("rebind join");
    for _ in 0..100 {
        if matches!(host.status(), peers::PairingStatus::Paired { .. }) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(matches!(host.status(), peers::PairingStatus::Paired { .. }));
}

#[test]
fn post_restore_creations_survive_and_deletions_stay_dead() {
    let (db_a, _dir_a) = open_db();
    let (db_b, dir_b) = open_db();
    let work = tempdir().expect("work");
    let (id_a, id_b) = pair(&db_a, &db_b);

    let n1 = make_note(&db_b, "n1", "created before the backup");
    sync_once(&db_a, &db_b, &id_a);
    let dirs = export_archive_of(dir_b.path(), work.path());

    let n2 = make_note(&db_b, "n2", "created after the backup");
    sync_once(&db_a, &db_b, &id_a);
    db_a.delete_note(&n1).expect("delete n1");
    sync_once(&db_a, &db_b, &id_a);
    let report = gc::run_gc(&db_a).expect("gc");
    assert!(report.purged >= 1, "n1 tombstone must be GC'd on A");
    assert!(!meta_exists(&db_a, "note", &n1));

    let db_b = restore_device(db_b, &dirs);
    assert!(db_b.get_note(&n1).expect("q").is_some(), "restore sanity");
    assert!(db_b.get_note(&n2).expect("q").is_none(), "restore sanity");
    assert!(
        db_b.get_setting("openai_api_key").is_none(),
        "secrets must not come back from the archive"
    );
    assert_eq!(
        db_b.list_peers().expect("peers").len(),
        0,
        "pairings must not come back from the archive"
    );

    let n_new = make_note(&db_b, "fresh", "written right after the restore");

    let watermark_before: i64 = db_a
        .conn()
        .query_row(
            "SELECT last_acked_seq FROM sync_peers WHERE device_id = ?1",
            [&id_b],
            |r| r.get(0),
        )
        .expect("watermark");
    repair_with_rebind(&db_a, &db_b, &id_b);
    let watermark_after: i64 = db_a
        .conn()
        .query_row(
            "SELECT last_acked_seq FROM sync_peers WHERE device_id = ?1",
            [&id_b],
            |r| r.get(0),
        )
        .expect("watermark");
    assert_eq!(
        watermark_before, watermark_after,
        "rebind must preserve the retained watermark (native T19 detections)"
    );

    sync_once(&db_a, &db_b, &id_a);
    assert!(db_a.get_note(&n1).expect("q").is_none(), "no resurrection");
    assert!(db_b.get_note(&n1).expect("q").is_none(), "no resurrection");
    assert!(
        db_b.get_note(&n2).expect("q").is_some(),
        "lost data returns"
    );
    assert!(
        db_b.get_note(&n_new).expect("q").is_some(),
        "post-restore creation must survive on the restored device"
    );
    assert!(
        db_a.get_note(&n_new).expect("q").is_some(),
        "post-restore creation must reach the peer"
    );
    let archived: i64 = db_a
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sync_conflicts WHERE entity_id = ?1",
            [&n1],
            |r| r.get(0),
        )
        .expect("conflict count");
    assert_eq!(archived, 1, "re-deleted content must be archived");

    assert_eq!(
        db_b.get_setting("sync_restored_pending"),
        None,
        "marker cleared once every peer is settled"
    );

    let (h, c) = sync_once(&db_a, &db_b, &id_a);
    assert_eq!(h.conflicts + c.conflicts, 0, "converged, no conflict churn");
}

#[test]
fn restored_marker_clears_only_after_every_peer_settled() {
    let (db_b, dir_b) = open_db();
    let (db_a, _dir_a) = open_db();
    let (db_c, _dir_c) = open_db();
    let work = tempdir().expect("work");
    let (id_b_for_a, _id_a) = pair(&db_b, &db_a);
    let (id_b_for_c, _id_c) = pair(&db_b, &db_c);
    assert_eq!(id_b_for_a, id_b_for_c);
    let id_b = id_b_for_a;

    let n1 = make_note(&db_b, "n1", "everywhere before the backup");
    sync_once(&db_b, &db_a, &id_b);
    sync_once(&db_b, &db_c, &id_b);
    let dirs = export_archive_of(dir_b.path(), work.path());

    db_b.delete_note(&n1).expect("delete");
    sync_once(&db_b, &db_a, &id_b);
    sync_once(&db_b, &db_c, &id_b);
    sync_once(&db_b, &db_a, &id_b);
    gc::run_gc(&db_a).expect("gc a");
    gc::run_gc(&db_c).expect("gc c");
    assert!(!meta_exists(&db_a, "note", &n1), "A tombstone GC'd");
    assert!(!meta_exists(&db_c, "note", &n1), "C tombstone GC'd");

    let db_b = restore_device(db_b, &dirs);
    assert!(db_b.get_note(&n1).expect("q").is_some(), "restore sanity");
    repair_with_rebind(&db_a, &db_b, &id_b);
    repair_with_rebind(&db_c, &db_b, &id_b);

    sync_once(
        &db_a,
        &db_b,
        &peers::ensure_sync_identity(&db_a).unwrap().device_id,
    );
    assert!(db_b.get_note(&n1).expect("q").is_none(), "A re-deleted n1");
    assert_eq!(
        db_b.get_setting("sync_restored_pending").as_deref(),
        Some("true"),
        "marker must survive until C is settled too"
    );

    sync_once(
        &db_c,
        &db_b,
        &peers::ensure_sync_identity(&db_c).unwrap().device_id,
    );
    assert!(
        db_c.get_note(&n1).expect("q").is_none(),
        "no resurrection on C"
    );
    assert_eq!(
        db_b.get_setting("sync_restored_pending"),
        None,
        "marker cleared after the LAST peer settled"
    );
}

#[test]
fn hlc_guard_keeps_the_fresh_edit_and_archives_the_stale_winner() {
    let (db_a, _dir_a) = open_db();
    let (db_b, dir_b) = open_db();
    let work = tempdir().expect("work");
    let (id_a, id_b) = pair(&db_a, &db_b);

    let n1 = make_note(&db_b, "note", "v1 original");
    sync_once(&db_a, &db_b, &id_a);
    let dirs = export_archive_of(dir_b.path(), work.path());

    db_b.update_note(
        &n1,
        &UpdateNote {
            title: None,
            content: Some("v2 pre-loss".into()),
            tags: None,
        },
    )
    .expect("edit v2");
    db_b.update_note(
        &n1,
        &UpdateNote {
            title: None,
            content: Some("v3 pre-loss".into()),
            tags: None,
        },
    )
    .expect("edit v3");
    sync_once(&db_a, &db_b, &id_a);
    assert_eq!(
        db_a.get_note(&n1).expect("q").expect("row").content,
        "v3 pre-loss"
    );

    let db_b = restore_device(db_b, &dirs);
    std::thread::sleep(std::time::Duration::from_millis(5));
    db_b.update_note(
        &n1,
        &UpdateNote {
            title: None,
            content: Some("fresh edit after restore".into()),
            tags: None,
        },
    )
    .expect("fresh edit");
    repair_with_rebind(&db_a, &db_b, &id_b);

    sync_once(&db_a, &db_b, &id_a);
    assert_eq!(
        db_a.get_note(&n1).expect("q").expect("row").content,
        "fresh edit after restore",
        "the fresher edit must win on A (not silently skipped)"
    );
    assert_eq!(
        db_b.get_note(&n1).expect("q").expect("row").content,
        "fresh edit after restore",
        "the fresher edit must survive on B (not silently overwritten)"
    );
    let archived_a: i64 = db_a
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sync_conflicts WHERE entity_id = ?1",
            [&n1],
            |r| r.get(0),
        )
        .expect("conflicts");
    let archived_b: i64 = db_b
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sync_conflicts WHERE entity_id = ?1",
            [&n1],
            |r| r.get(0),
        )
        .expect("conflicts");
    assert!(
        archived_a + archived_b >= 1,
        "the losing stale version must be archived, never dropped"
    );
}

fn chunk_record(note_id: &str, text: &str) -> ChunkRecord {
    let mut vector = vec![0.0_f32; EMBEDDING_DIMS];
    vector[0] = 1.0;
    ChunkRecord {
        id: format!("note:{note_id}:0"),
        owner_id: note_id.to_string(),
        owner_kind: "note".to_string(),
        chunk_index: 0,
        embed_profile: flowflow::application::constants::EMBEDDING_PROFILE
            .to_string(),
        vector,
        content_hash: "h".to_string(),
        chunk_text: text.to_string(),
        title: "t".to_string(),
        tags: "[]".to_string(),
        created_at: "2026-06-11T00:00:00Z".to_string(),
    }
}

#[tokio::test]
async fn round_trip_over_divergent_vectordb_rebuilds_backup_content() {
    let _env = VDB_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let vdb_dir = tempdir().expect("vdb dir");
    std::env::set_var("FLOWFLOW_VECTORDB_PATH", vdb_dir.path());
    let dir = tempdir().expect("db dir");
    let work = tempdir().expect("work dir");
    let db = Database::open_at(dir.path().join("flowflow.db")).expect("open");

    let note_id = make_note(&db, "n", "the original pre-backup content");
    db.replace_chunks(
        &note_id,
        "note",
        &[chunk_record(&note_id, "the original pre-backup content")],
    )
    .expect("seed chunks");
    db.set_setting("chunks_backfilled_v10", "true")
        .expect("flag");
    let store = VectorStore::open().await.expect("store");
    reconcile_once(&db, &store).await.expect("mirror to lance");

    let mut dirs = export_archive_of(dir.path(), work.path());
    dirs.paths.vectordb_dir = vdb_dir.path().to_path_buf();

    db.replace_chunks(
        &note_id,
        "note",
        &[chunk_record(&note_id, "DIVERGED after the backup")],
    )
    .expect("diverge chunks");
    reconcile_once(&db, &store)
        .await
        .expect("mirror divergence");
    drop(store);

    let db = restore_device(Arc::new(db), &dirs);
    assert!(
        !vdb_dir.path().exists(),
        "the derived vectordb cache must be purged at the swap"
    );

    let store = VectorStore::open().await.expect("fresh store");
    backfill_legacy_chunks(&db, &store).await.expect("backfill");
    reconcile_once(&db, &store).await.expect("rebuild");
    let rows = store.fetch_note_rows(&note_id).await.expect("rows");
    assert_eq!(rows.len(), 1, "index rebuilt from the restored BLOBs");
    assert_eq!(
        rows[0].chunk_text, "the original pre-backup content",
        "search must serve the BACKUP content, not the post-backup leftovers"
    );
    std::env::remove_var("FLOWFLOW_VECTORDB_PATH");
}

#[test]
fn rebind_clears_the_ack_book_and_requires_one_authorization_per_attempt() {
    let (db_a, _dir_a) = open_db();
    let (db_b, dir_b) = open_db();
    let work = tempdir().expect("work");
    let (id_a, id_b) = pair(&db_a, &db_b);

    make_note(&db_b, "n", "give the ack book something to record");
    sync_once(&db_a, &db_b, &id_a);
    let ack_key = format!("sync_peer_acked_by_{id_b}");
    assert!(
        db_a.get_setting(&ack_key).is_some(),
        "ack book must be primed before the rebind"
    );

    let dirs = export_archive_of(dir_b.path(), work.path());
    let db_b = restore_device(db_b, &dirs);
    repair_with_rebind(&db_a, &db_b, &id_b);

    assert_eq!(
        db_a.get_setting(&ack_key).as_deref().unwrap_or(""),
        "",
        "rebind must clear the stale ack book entry"
    );
    assert_eq!(
        db_a.get_setting(&format!("sync_rebind_ok_{id_b}"))
            .as_deref(),
        Some(""),
        "the rebind authorization must be consumed (one-shot)"
    );
    assert!(
        db_a.get_setting(&format!("sync_rebind_at_{id_b}"))
            .is_some(),
        "the rotation timestamp must be recorded"
    );
}
