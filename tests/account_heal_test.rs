use flowflow::application::account_heal;
use flowflow::domain::note::NewTextNote;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::engine::{set_peer_endpoint, SyncEngine};
use flowflow::infrastructure::sync::peers;
use std::sync::{Arc, Once};
use tempfile::tempdir;

static ADVERTISE: Once = Once::new();

// Unreachable backend: connection refused fast, so the heal path runs its
// real code (client built, HTTP attempted) without a live server.
const DEAD_BACKEND: &str = "http://127.0.0.1:9";

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

#[test]
fn join_request_signature_binds_to_the_session_hash() {
    let (db, _d) = open_db();
    let hash = [7u8; 32];
    let (pubkey, sig) =
        account_heal::sign_join_request(&db, &hash).expect("sign");
    assert!(account_heal::verify_join_request(&pubkey, &sig, &hash));
    // Another session's hash must not verify: no cross-session replay.
    assert!(!account_heal::verify_join_request(
        &pubkey, &sig, &[8u8; 32]
    ));
    assert!(!account_heal::verify_join_request(
        &pubkey,
        "bm90YXNpZw==",
        &hash
    ));
    assert!(!account_heal::verify_join_request(
        "notbase64!",
        &sig,
        &hash
    ));
}

#[test]
fn heal_rule_only_fires_free_toward_premium_on_the_same_backend() {
    let (db, _d) = open_db();
    db.set_setting("backend_base_url", DEAD_BACKEND).unwrap();

    // Peer not premium: nothing to adopt.
    assert!(!account_heal::wants_join(&db, false, Some(DEAD_BACKEND)));
    // Peer premium, same backend: ask.
    assert!(account_heal::wants_join(&db, true, Some(DEAD_BACKEND)));
    // Different backends: refuse and surface why.
    assert!(!account_heal::wants_join(&db, true, Some("http://other:1")));
    let err = db
        .get_setting(account_heal::HEAL_ERROR_KEY)
        .unwrap_or_default();
    assert!(err.contains("different backends"), "visible reason: {err}");
    // Already premium: never move.
    db.set_setting(account_heal::PREMIUM_CACHE_KEY, "true")
        .unwrap();
    assert!(!account_heal::wants_join(&db, true, Some(DEAD_BACKEND)));
}

// The heal frames ride a real session between two paired DBs with a dead
// backend. The mint side answers token: None (invite unreachable), and the
// session itself must still sync notes - a heal failure is never a sync
// failure.
#[test]
fn session_survives_heal_exchange_with_dead_backend() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    db_a.set_setting("backend_base_url", DEAD_BACKEND).unwrap();
    db_b.set_setting("backend_base_url", DEAD_BACKEND).unwrap();
    // A believes it is premium: B will ask, A will try to mint and fail.
    db_a.set_setting(account_heal::PREMIUM_CACHE_KEY, "true")
        .unwrap();

    let engine_a = SyncEngine::start_listener(db_a.clone(), 0);
    let port_a = engine_a.listen_port().expect("listener bound");
    set_peer_endpoint(&db_b, &id_a, "127.0.0.1", port_a).expect("endpoint");

    let note_a = db_a
        .create_text_note(&NewTextNote {
            title: Some("from A".into()),
            content: "premium side note".into(),
            tags: vec![],
        })
        .expect("create note")
        .id;

    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    engine_b.sync_now_blocking();

    // A concurrent beacon-triggered pass can hold the session lock and make
    // sync_now_blocking a no-op; poll for the outcome instead.
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if db_b.get_note(&note_a).expect("q").is_some() {
            break;
        }
        engine_b.sync_now_blocking();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        db_b.get_note(&note_a).expect("q").is_some(),
        "notes must sync even when the heal mint fails"
    );
    assert!(
        !account_heal::my_premium_cached(&db_b),
        "no token minted, B must stay free"
    );
    assert!(
        db_b.get_setting(account_heal::HEAL_EVENT_KEY)
            .unwrap_or_default()
            .is_empty(),
        "no adoption event without a redeemed token"
    );
}
