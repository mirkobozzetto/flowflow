use flowflow::domain::note::NewTextNote;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::sync::engine::{
    clear_peer_endpoint, peer_endpoint, set_peer_endpoint, SyncActivity,
    SyncEngine,
};
use flowflow::infrastructure::sync::peers;
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
        // The engine's post-apply reconcile thread opens the GLOBAL store
        // (Database::open). On macOS that now resolves to the real
        // Application Support dir - point it at a scratch dir instead so a
        // test run never touches actual user data.
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
fn pairing_seeds_the_peer_address_book() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, id_b) = pair(&db_a, &db_b);

    let joiner_side = peer_endpoint(&db_a, &id_b);
    let host_side = peer_endpoint(&db_b, &id_a);
    assert!(
        joiner_side.is_some(),
        "host must learn the joiner's address from the pairing socket"
    );
    assert!(
        host_side.is_some(),
        "joiner must learn the host's address from the payload"
    );
    assert_eq!(host_side.unwrap().0, "127.0.0.1");

    peers::unpair(&db_a, &id_b).expect("unpair");
    assert!(
        peer_endpoint(&db_a, &id_b).is_none(),
        "unpair must drop the stored endpoint"
    );
}

#[test]
fn manual_sync_flows_through_the_listener_and_reports_done() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, id_b) = pair(&db_a, &db_b);

    let engine_a = SyncEngine::start_listener(db_a.clone(), 0);
    let port_a = engine_a.listen_port().expect("listener bound");
    set_peer_endpoint(&db_b, &id_a, "127.0.0.1", port_a).expect("endpoint");
    // Pairing seeded A's address book; wipe it so the final assertion can
    // only pass if the SERVED session re-learns the address (DHCP refresh).
    clear_peer_endpoint(&db_a, &id_b);

    let note_b = make_note(&db_b, "from B", "engine carried me");
    let note_a = make_note(&db_a, "from A", "engine carried me back");

    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    engine_b.sync_now_blocking();

    assert!(db_a.get_note(&note_b).expect("q").is_some());
    assert!(db_b.get_note(&note_a).expect("q").is_some());

    assert!(
        matches!(engine_b.activity(), SyncActivity::Done { .. }),
        "outbound side reports Done, got {:?}",
        engine_b.activity()
    );
    assert!(
        wait_until(3000, || matches!(
            engine_a.activity(),
            SyncActivity::Done { .. }
        )),
        "inbound side reports Done, got {:?}",
        engine_a.activity()
    );
    assert!(
        wait_until(3000, || peer_endpoint(&db_a, &id_b).is_some()),
        "served session must refresh the peer's address"
    );
}

// v4: author_device rides the note catalog verbatim (no re-authoring on
// apply), and the HELLO device_name lands on the receiving peer row.
#[test]
fn sync_carries_note_author_and_peer_name() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, id_b) = pair(&db_a, &db_b);
    db_a.set_setting("device_name", "iPhone de Mirko").unwrap();

    let engine_a = SyncEngine::start_listener(db_a.clone(), 0);
    let port_a = engine_a.listen_port().expect("listener bound");
    set_peer_endpoint(&db_b, &id_a, "127.0.0.1", port_a).expect("endpoint");

    let note_a = make_note(&db_a, "from A", "authored on A");
    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    engine_b.sync_now_blocking();

    let copy = db_b.get_note(&note_a).expect("q").expect("note synced");
    assert_eq!(
        copy.author_device.as_deref(),
        Some(id_a.as_str()),
        "author must survive the wire, not be re-stamped by the receiver"
    );
    let peer = db_b.get_peer(&id_a).expect("q").expect("peer row");
    assert_eq!(peer.name.as_deref(), Some("iPhone de Mirko"));
    // B never chose a name: the generated default ("adjective-animal-NN")
    // travels instead, so a peer label is never empty.
    let peer_b = db_a.get_peer(&id_b).expect("q").expect("peer row");
    let generated = peer_b.name.expect("generated default name");
    assert!(!generated.is_empty());
    assert_eq!(generated.split('-').count(), 3, "docker-style: {generated}");
}

// The network-change deadlock: both address books stale at once, nobody can
// dial anybody. A single beacon announce must rewrite the stale endpoint
// from the packet's source and let the next pass reach the peer.
#[test]
fn beacon_heals_stale_endpoints_and_resyncs() {
    use flowflow::infrastructure::sync::beacon;

    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, id_b) = pair(&db_a, &db_b);

    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    let port_b = engine_b.listen_port().expect("listener bound");
    let note_b = make_note(&db_b, "from B", "found me through the beacon");

    // Both books poisoned: the exact both-sides-stale deadlock.
    set_peer_endpoint(&db_a, &id_b, "192.0.2.1", 1).expect("endpoint");
    set_peer_endpoint(&db_b, &id_a, "192.0.2.2", 1).expect("endpoint");

    // A's beacon listener on an ephemeral UDP port (tests must not race for
    // the well-known one).
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").expect("udp bind");
    let udp_port = udp.local_addr().unwrap().port();
    beacon::spawn_listener_on(udp, db_a.clone());

    // B announces itself to A.
    beacon::announce_to(("127.0.0.1", udp_port), &id_b, port_b)
        .expect("announce");

    assert!(
        wait_until(3000, || {
            peer_endpoint(&db_a, &id_b)
                .is_some_and(|(h, p)| h == "127.0.0.1" && p == port_b)
        }),
        "beacon must rewrite the stale endpoint from the packet source"
    );

    let engine_a = SyncEngine::start_listener(db_a.clone(), 0);
    engine_a.sync_now_blocking();
    assert!(
        db_a.get_note(&note_b).expect("q").is_some(),
        "healed endpoint must carry the note"
    );
}

#[test]
fn debounced_save_syncs_after_the_quiet_period() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let engine_a = SyncEngine::start_listener(db_a.clone(), 0);
    let port_a = engine_a.listen_port().expect("listener bound");
    set_peer_endpoint(&db_b, &id_a, "127.0.0.1", port_a).expect("endpoint");

    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    let note = make_note(&db_b, "debounced", "saved then synced");
    engine_b.schedule_debounced();
    engine_b.schedule_debounced();

    assert!(
        db_a.get_note(&note).expect("q").is_none(),
        "nothing flows before the quiet period"
    );
    assert!(
        wait_until(8000, || db_a.get_note(&note).expect("q").is_some()),
        "the debounced sync must deliver the note"
    );
}

#[test]
fn unreachable_peer_is_a_visible_error_not_a_silent_stall() {
    let (db_a, _da) = open_db();
    let (db_b, _db) = open_db();
    let (id_a, _id_b) = pair(&db_a, &db_b);

    let dead = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let dead_port = dead.local_addr().expect("addr").port();
    drop(dead);
    set_peer_endpoint(&db_b, &id_a, "127.0.0.1", dead_port).expect("endpoint");

    let engine_b = SyncEngine::start_listener(db_b.clone(), 0);
    engine_b.sync_now_blocking();

    assert!(
        matches!(engine_b.activity(), SyncActivity::Error { .. }),
        "a sync that cannot progress must surface, got {:?}",
        engine_b.activity()
    );
}
