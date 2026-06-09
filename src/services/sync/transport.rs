use super::SyncError;
use snow::params::NoiseParams;
use snow::{Builder, HandshakeState, Keypair};

pub const NOISE_PARAMS: &str = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s";
pub const PSK_LEN: usize = 32;
pub const MAX_NOISE_MESSAGE: usize = 65535;

fn params() -> Result<NoiseParams, SyncError> {
    NOISE_PARAMS
        .parse()
        .map_err(|e| SyncError::Handshake(format!("noise params: {e:?}")))
}

pub fn generate_static_keypair() -> Result<Keypair, SyncError> {
    Builder::new(params()?)
        .generate_keypair()
        .map_err(|e| SyncError::Handshake(format!("keypair: {e}")))
}

pub fn build_initiator(
    local_private: &[u8],
    psk: &[u8; PSK_LEN],
) -> Result<HandshakeState, SyncError> {
    Builder::new(params()?)
        .local_private_key(local_private)
        .map_err(|e| SyncError::Handshake(format!("private key: {e}")))?
        .psk(3, psk)
        .map_err(|e| SyncError::Handshake(format!("psk: {e}")))?
        .build_initiator()
        .map_err(|e| SyncError::Handshake(format!("initiator: {e}")))
}

pub fn build_responder(
    local_private: &[u8],
    psk: &[u8; PSK_LEN],
) -> Result<HandshakeState, SyncError> {
    Builder::new(params()?)
        .local_private_key(local_private)
        .map_err(|e| SyncError::Handshake(format!("private key: {e}")))?
        .psk(3, psk)
        .map_err(|e| SyncError::Handshake(format!("psk: {e}")))?
        .build_responder()
        .map_err(|e| SyncError::Handshake(format!("responder: {e}")))
}

pub fn inmemory_handshake_with_psks(
    initiator_psk: &[u8; PSK_LEN],
    responder_psk: &[u8; PSK_LEN],
    plaintext: &[u8],
) -> Result<Vec<u8>, SyncError> {
    let init_kp = generate_static_keypair()?;
    let resp_kp = generate_static_keypair()?;

    let mut initiator = build_initiator(&init_kp.private, initiator_psk)?;
    let mut responder = build_responder(&resp_kp.private, responder_psk)?;

    let mut buf = vec![0u8; MAX_NOISE_MESSAGE];
    let mut read = vec![0u8; MAX_NOISE_MESSAGE];

    let n = initiator
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg1 write: {e}")))?;
    responder
        .read_message(&buf[..n], &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg1 read: {e}")))?;

    let n = responder
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg2 write: {e}")))?;
    initiator
        .read_message(&buf[..n], &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg2 read: {e}")))?;

    let n = initiator
        .write_message(&[], &mut buf)
        .map_err(|e| SyncError::Handshake(format!("msg3 write: {e}")))?;
    responder
        .read_message(&buf[..n], &mut read)
        .map_err(|e| SyncError::Handshake(format!("msg3 read: {e}")))?;

    let mut initiator = initiator
        .into_transport_mode()
        .map_err(|e| SyncError::Transport(format!("init transport: {e}")))?;
    let mut responder = responder
        .into_transport_mode()
        .map_err(|e| SyncError::Transport(format!("resp transport: {e}")))?;

    let n = initiator
        .write_message(plaintext, &mut buf)
        .map_err(|e| SyncError::Transport(format!("encrypt: {e}")))?;
    let m = responder
        .read_message(&buf[..n], &mut read)
        .map_err(|e| SyncError::Transport(format!("decrypt: {e}")))?;

    Ok(read[..m].to_vec())
}

pub fn inmemory_handshake_roundtrip(
    plaintext: &[u8],
) -> Result<Vec<u8>, SyncError> {
    let psk = super::peers::generate_psk()?;
    inmemory_handshake_with_psks(&psk, &psk, plaintext)
}
