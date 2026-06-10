use flowflow::db::Database;
use flowflow::services::sync::{peers, transport};
use std::sync::{Arc, Once};
use tempfile::tempdir;

// Advertise loopback so start_pairing_host does not depend on a default route
// (lets the host tests run offline / in sandboxed CI). Set once, same value.
static ADVERTISE: Once = Once::new();

fn host_loopback(db: Arc<Database>) -> peers::PairingHost {
    ADVERTISE.call_once(|| {
        std::env::set_var("FLOWFLOW_SYNC_ADVERTISE_ADDR", "127.0.0.1");
    });
    peers::start_pairing_host(db).expect("host")
}

fn open_db() -> (Arc<Database>, tempfile::TempDir) {
    let dir = tempdir().expect("db tempdir");
    let db =
        Database::open_at(dir.path().join("flowflow.db")).expect("open_at");
    (Arc::new(db), dir)
}

fn localhost_uri(host: &peers::PairingHost) -> String {
    let mut payload =
        peers::decode_pairing_uri(&host.uri).expect("decode host uri");
    payload.addr = "127.0.0.1".to_string();
    peers::encode_pairing_uri(&payload).expect("re-encode uri")
}

fn wait_for_terminal_status(host: &peers::PairingHost) -> peers::PairingStatus {
    for _ in 0..100 {
        let s = host.status();
        if s != peers::PairingStatus::Waiting {
            return s;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    host.status()
}

#[test]
fn pairing_payload_roundtrips_through_uri() {
    let kp = transport::generate_static_keypair().expect("keypair");
    let payload = peers::new_pairing_payload(
        "device-abc".to_string(),
        "192.168.1.42".to_string(),
        8765,
        kp.public.clone(),
    )
    .expect("payload");

    let uri = peers::encode_pairing_uri(&payload).expect("encode");
    assert!(uri.starts_with(peers::PAIRING_SCHEME));

    let decoded = peers::decode_pairing_uri(&uri).expect("decode");
    assert_eq!(decoded, payload);
}

#[test]
fn pairing_uri_fits_in_a_qr_code() {
    let kp = transport::generate_static_keypair().expect("keypair");
    let payload =
        peers::new_pairing_payload("d".into(), "10.0.0.1".into(), 1, kp.public)
            .expect("payload");
    let uri = peers::encode_pairing_uri(&payload).expect("encode");
    let svg = peers::pairing_qr_svg(&uri).expect("qr");
    assert!(svg.contains("<svg"));
}

#[test]
fn decode_rejects_foreign_scheme() {
    assert!(peers::decode_pairing_uri("https://evil/#abc").is_err());
}

#[test]
fn generated_psks_differ() {
    let a = peers::generate_psk().expect("psk a");
    let b = peers::generate_psk().expect("psk b");
    assert_ne!(a, b);
}

#[test]
fn manual_addr_parses_host_and_port() {
    let (host, port) =
        peers::parse_manual_addr("192.168.1.5:8765").expect("parse");
    assert_eq!(host, "192.168.1.5");
    assert_eq!(port, 8765);
    assert!(peers::parse_manual_addr("noport").is_err());
    assert!(peers::parse_manual_addr(":8765").is_err());
}

#[test]
fn sync_identity_is_stable_across_calls() {
    let (db, _dir) = open_db();
    let a = peers::ensure_sync_identity(&db).expect("identity a");
    let b = peers::ensure_sync_identity(&db).expect("identity b");
    assert_eq!(a.device_id, b.device_id);
    assert_eq!(a.public_key, b.public_key);
    assert_eq!(a.private_key, b.private_key);
    assert_eq!(a.public_key.len(), 32);
}

#[test]
fn host_and_join_pair_both_databases() {
    let (db_a, _dir_a) = open_db();
    let (db_b, _dir_b) = open_db();

    let host = host_loopback(db_a.clone());
    let uri = localhost_uri(&host);
    let host_device_id =
        peers::join_pairing(&db_b, &uri).expect("join pairing");

    let status = wait_for_terminal_status(&host);
    let id_a = peers::ensure_sync_identity(&db_a).expect("id a").device_id;
    let id_b = peers::ensure_sync_identity(&db_b).expect("id b").device_id;
    assert_eq!(host_device_id, id_a);
    assert_eq!(
        status,
        peers::PairingStatus::Paired {
            device_id: id_b.clone()
        }
    );

    let peer_on_a = db_a.get_peer(&id_b).expect("peer query").expect("row");
    let peer_on_b = db_b.get_peer(&id_a).expect("peer query").expect("row");
    let pub_a = peers::ensure_sync_identity(&db_a).expect("id").public_key;
    let pub_b = peers::ensure_sync_identity(&db_b).expect("id").public_key;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    assert_eq!(peer_on_a.static_pubkey, URL_SAFE_NO_PAD.encode(&pub_b));
    assert_eq!(peer_on_b.static_pubkey, URL_SAFE_NO_PAD.encode(&pub_a));

    let psk_a = peers::load_peer_psk(&db_a, &id_b).expect("psk a");
    let psk_b = peers::load_peer_psk(&db_b, &id_a).expect("psk b");
    assert!(psk_a.is_some());
    assert_eq!(psk_a, psk_b);
}

#[test]
fn join_with_tampered_psk_is_refused_and_persists_nothing() {
    let (db_a, _dir_a) = open_db();
    let (db_b, _dir_b) = open_db();

    let host = host_loopback(db_a.clone());
    let mut payload =
        peers::decode_pairing_uri(&host.uri).expect("decode host uri");
    payload.addr = "127.0.0.1".to_string();
    payload.psk = peers::generate_psk().expect("other psk");
    let uri = peers::encode_pairing_uri(&payload).expect("re-encode");

    let res = peers::join_pairing(&db_b, &uri);
    assert!(res.is_err(), "tampered PSK must refuse pairing");
    let id_a = peers::ensure_sync_identity(&db_a).expect("id a").device_id;
    assert!(db_b.get_peer(&id_a).expect("query").is_none());
    assert!(db_a.list_peers().expect("list").is_empty());
    host.cancel();
}

#[test]
fn join_with_tampered_fingerprint_is_refused() {
    let (db_a, _dir_a) = open_db();
    let (db_b, _dir_b) = open_db();

    let host = host_loopback(db_a.clone());
    let mut payload =
        peers::decode_pairing_uri(&host.uri).expect("decode host uri");
    payload.addr = "127.0.0.1".to_string();
    payload.static_pubkey =
        transport::generate_static_keypair().expect("kp").public;
    let uri = peers::encode_pairing_uri(&payload).expect("re-encode");

    let res = peers::join_pairing(&db_b, &uri);
    match res {
        Err(e) => assert!(
            e.to_string().contains("fingerprint"),
            "expected fingerprint refusal, got: {e}"
        ),
        Ok(_) => panic!("tampered fingerprint must refuse pairing"),
    }
    assert!(db_b.list_peers().expect("list").is_empty());
    host.cancel();
}

#[test]
fn join_refuses_to_overwrite_existing_peer_with_different_key() {
    let (db_a, _dir_a) = open_db();
    let (db_b, _dir_b) = open_db();

    let host = host_loopback(db_a.clone());
    let uri = localhost_uri(&host);
    let id_a = peers::join_pairing(&db_b, &uri).expect("first join");
    wait_for_terminal_status(&host);

    let peer_before = db_b.get_peer(&id_a).expect("query").expect("row");

    let host2 = host_loopback(db_a.clone());
    let mut payload2 = peers::decode_pairing_uri(&host2.uri).expect("decode");
    payload2.addr = "127.0.0.1".to_string();
    payload2.device_id = id_a.clone();
    payload2.static_pubkey =
        transport::generate_static_keypair().expect("kp").public;
    let uri2 = peers::encode_pairing_uri(&payload2).expect("re-encode");

    let res = peers::join_pairing(&db_b, &uri2);
    assert!(
        res.is_err(),
        "must refuse a different key for the same device_id"
    );
    let peer_after = db_b.get_peer(&id_a).expect("query").expect("row");
    assert_eq!(
        peer_after.static_pubkey, peer_before.static_pubkey,
        "existing binding must be preserved"
    );
    host2.cancel();
}

#[test]
fn unpair_removes_row_and_psk() {
    let (db_a, _dir_a) = open_db();
    let (db_b, _dir_b) = open_db();

    let host = host_loopback(db_a.clone());
    let uri = localhost_uri(&host);
    let id_a = peers::join_pairing(&db_b, &uri).expect("join");
    wait_for_terminal_status(&host);

    peers::unpair(&db_b, &id_a).expect("unpair");
    assert!(db_b.get_peer(&id_a).expect("query").is_none());
    assert!(peers::load_peer_psk(&db_b, &id_a).expect("psk").is_none());
}
