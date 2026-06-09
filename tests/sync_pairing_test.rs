use flowflow::services::sync::{peers, transport};

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
