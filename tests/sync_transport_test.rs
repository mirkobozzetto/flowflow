use flowflow::services::sync::transport;

#[test]
fn xxpsk3_handshake_roundtrip_succeeds() {
    let msg = b"flowflow sync spike payload";
    let out = transport::inmemory_handshake_roundtrip(msg).expect("handshake");
    assert_eq!(out, msg);
}

#[test]
fn xxpsk3_handshake_rejects_mismatched_psk() {
    let a = [1u8; 32];
    let b = [2u8; 32];
    let res = transport::inmemory_handshake_with_psks(&a, &b, b"x");
    assert!(res.is_err(), "mismatched PSK must fail the handshake");
}

#[test]
fn static_keypair_has_curve25519_length() {
    let kp = transport::generate_static_keypair().expect("keypair");
    assert_eq!(kp.public.len(), 32);
    assert_eq!(kp.private.len(), 32);
}
