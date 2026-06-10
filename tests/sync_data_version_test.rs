use flowflow::db::Database;
use flowflow::models::note::NewTextNote;
use flowflow::services::sync::engine::{
    set_peer_endpoint, SyncActivity, SyncEngine,
};
use flowflow::services::sync::peers;
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
        let scratch = std::env::temp_dir().join("flowflow-test-data");
        std::env::set_var("FLOWFLOW_DATA_DIR", &scratch);
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

fn make_note(db: &Database, title: &str, content: &str) -> String {
    db.create_text_note(&NewTextNote {
        title: Some(title.to_string()),
        content: content.to_string(),
        tags: vec![],
    })
    .expect("create note")
    .id
}

fn wait_until(timeout_ms: u64, mut probe: impl FnMut() -> bool) -> bool {
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_millis(timeout_ms);
    while std::time::Instant::now() < deadline {
        if probe() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

#[test]
fn data_version_bumps_on_outbound_apply() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let engine_a = SyncEngine::start_listener(db_a.clone(), 0);
    let port_a = engine_a.listen_port().expect("listener bound");
    set_peer_endpoint(&db_b, &id_a, "127.0.0.1", port_a).expect("endpoint");

    make_note(&db_a, "from A", "carried back to B");

    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    let before = engine_b.data_version();
    engine_b.sync_now_blocking();

    assert!(
        matches!(engine_b.activity(), SyncActivity::Done { .. }),
        "outbound side reports Done, got {:?}",
        engine_b.activity()
    );
    assert!(
        engine_b.data_version() > before,
        "outbound apply must bump the data-version ({} -> {})",
        before,
        engine_b.data_version()
    );
}

#[test]
fn data_version_bumps_on_served_apply() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let engine_a = SyncEngine::start_listener(db_a.clone(), 0);
    let port_a = engine_a.listen_port().expect("listener bound");
    set_peer_endpoint(&db_b, &id_a, "127.0.0.1", port_a).expect("endpoint");

    let before_a = engine_a.data_version();

    let note_b = make_note(&db_b, "from B", "served into A");

    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    engine_b.sync_now_blocking();

    assert!(
        wait_until(3000, || db_a.get_note(&note_b).expect("q").is_some()),
        "served session must apply the pushed note"
    );
    assert!(
        wait_until(3000, || engine_a.data_version() > before_a),
        "served apply must bump the data-version ({} -> {})",
        before_a,
        engine_a.data_version()
    );
}

#[test]
fn data_version_steady_on_zero_change_pass() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let engine_a = SyncEngine::start_listener(db_a.clone(), 0);
    let port_a = engine_a.listen_port().expect("listener bound");
    set_peer_endpoint(&db_b, &id_a, "127.0.0.1", port_a).expect("endpoint");

    let note_b = make_note(&db_b, "seed", "converge first");

    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    engine_b.sync_now_blocking();
    assert!(
        wait_until(3000, || db_a.get_note(&note_b).expect("q").is_some()),
        "first pass must converge"
    );
    assert!(
        wait_until(3000, || engine_a.data_version() > 0),
        "first pass should have bumped A once"
    );

    assert!(
        wait_until(3000, || {
            !matches!(engine_a.activity(), SyncActivity::Syncing)
                && !matches!(engine_b.activity(), SyncActivity::Syncing)
        }),
        "both engines must settle before the steady snapshot"
    );
    let steady_a = engine_a.data_version();
    let steady_b = engine_b.data_version();

    engine_b.sync_now_blocking();
    std::thread::sleep(std::time::Duration::from_millis(500));

    assert_eq!(
        engine_b.data_version(),
        steady_b,
        "a zero-change outbound pass must not move the counter"
    );
    assert_eq!(
        engine_a.data_version(),
        steady_a,
        "a zero-change served pass must not move the counter"
    );
}
