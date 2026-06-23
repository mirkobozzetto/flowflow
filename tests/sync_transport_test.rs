use flowflow::infrastructure::sync::transport;
use std::net::TcpListener;

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

fn spawn_responder(
    psk: [u8; 32],
    expected_initiator: Option<Vec<u8>>,
) -> (
    u16,
    Vec<u8>,
    std::thread::JoinHandle<Result<Vec<Vec<u8>>, String>>,
) {
    let kp = transport::generate_static_keypair().expect("resp keypair");
    let pubkey = kp.public.clone();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().map_err(|e| e.to_string())?;
        let mut chan = transport::accept_secure(
            stream,
            &kp.private,
            &psk,
            expected_initiator.as_deref(),
        )
        .map_err(|e| e.to_string())?;
        let mut received = Vec::new();
        loop {
            match chan.recv() {
                Ok(msg) if msg == b"bye" => break,
                Ok(msg) => {
                    chan.send(&msg).map_err(|e| e.to_string())?;
                    received.push(msg);
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(received)
    });
    (port, pubkey, handle)
}

#[test]
fn tcp_channel_echoes_small_and_large_messages() {
    let psk = [7u8; 32];
    let (port, resp_pub, handle) = spawn_responder(psk, None);
    let kp = transport::generate_static_keypair().expect("init keypair");
    let mut chan = transport::connect_secure(
        "127.0.0.1",
        port,
        &kp.private,
        &psk,
        Some(&resp_pub),
    )
    .expect("secure connect");

    let small = b"hello flowflow".to_vec();
    let large = vec![0xABu8; 200_000];
    chan.send(&small).expect("send small");
    assert_eq!(chan.recv().expect("echo small"), small);
    chan.send(&large).expect("send large");
    assert_eq!(chan.recv().expect("echo large"), large);
    chan.send(b"bye").expect("send bye");

    let received = handle.join().expect("join").expect("responder ok");
    assert_eq!(received.len(), 2);
    assert_eq!(received[1].len(), 200_000);
}

#[test]
fn tcp_channel_rejects_wrong_psk() {
    let (port, resp_pub, handle) = spawn_responder([1u8; 32], None);
    let kp = transport::generate_static_keypair().expect("init keypair");
    let res = transport::connect_secure(
        "127.0.0.1",
        port,
        &kp.private,
        &[2u8; 32],
        Some(&resp_pub),
    );
    assert!(
        handle.join().expect("join").is_err(),
        "responder must reject msg3 encrypted with the wrong PSK"
    );
    if let Ok(mut chan) = res {
        let _ = chan.send(b"probe");
        assert!(
            chan.recv().is_err(),
            "initiator must never receive data with the wrong PSK"
        );
    }
}

#[test]
fn tcp_channel_rejects_unexpected_responder_fingerprint() {
    let psk = [3u8; 32];
    let (port, _resp_pub, handle) = spawn_responder(psk, None);
    let kp = transport::generate_static_keypair().expect("init keypair");
    let wrong = transport::generate_static_keypair().expect("wrong keypair");
    let res = transport::connect_secure(
        "127.0.0.1",
        port,
        &kp.private,
        &psk,
        Some(&wrong.public),
    );
    match res {
        Err(e) => assert!(
            e.to_string().contains("fingerprint"),
            "expected fingerprint refusal, got: {e}"
        ),
        Ok(_) => panic!("unexpected responder fingerprint must be refused"),
    }
    let _ = handle.join();
}

#[test]
fn tcp_responder_rejects_unexpected_initiator_fingerprint() {
    let psk = [4u8; 32];
    let wrong = transport::generate_static_keypair().expect("wrong keypair");
    let (port, resp_pub, handle) =
        spawn_responder(psk, Some(wrong.public.clone()));
    let kp = transport::generate_static_keypair().expect("init keypair");
    let mut chan = transport::connect_secure(
        "127.0.0.1",
        port,
        &kp.private,
        &psk,
        Some(&resp_pub),
    )
    .expect("initiator handshake completes before responder verdict");
    let err = chan.recv();
    assert!(err.is_err(), "responder must drop the channel");
    let resp = handle.join().expect("join");
    match resp {
        Err(e) => assert!(
            e.contains("fingerprint"),
            "expected fingerprint refusal, got: {e}"
        ),
        Ok(_) => panic!("responder must refuse unexpected initiator key"),
    }
}
