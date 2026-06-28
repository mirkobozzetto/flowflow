mod account_join;
mod codec;
mod host;
mod identity;
mod lan;
mod peer_store;

use super::transport;
use super::SyncError;
use crate::infrastructure::persistence::Database;
use account_join::redeem_join_token;
use host::{PairOk, PairRequest};
use peer_store::bind_peer;
use std::time::Duration;

pub use codec::{
    decode_pairing_uri, encode_pairing_uri, generate_psk, new_pairing_payload,
    pairing_qr_svg, parse_manual_addr, PairingPayload, PAIRING_SCHEME,
};
pub use host::{start_pairing_host, PairingHost, PairingStatus};
pub use identity::{
    ensure_sync_identity, SyncIdentity, STATIC_PRIVKEY_KEY, STATIC_PUBKEY_KEY,
};
pub use peer_store::{
    authorize_rebind, load_peer_psk, unpair, REBIND_REFUSED_MARKER,
};

pub const PSK_LEN: usize = 32;
pub const PAIRING_WINDOW: Duration = Duration::from_secs(300);

pub fn join_pairing(db: &Database, uri: &str) -> Result<String, SyncError> {
    let payload = decode_pairing_uri(uri.trim())?;
    let identity = ensure_sync_identity(db)?;
    if payload.device_id == identity.device_id {
        return Err(SyncError::Pairing("cannot pair with self".into()));
    }
    let mut chan = transport::connect_secure(
        &payload.addr,
        payload.port,
        &identity.private_key,
        &payload.psk,
        Some(&payload.static_pubkey),
    )?;
    let req = PairRequest {
        kind: "pair_request".into(),
        device_id: identity.device_id.clone(),
        backend_pubkey:
            crate::infrastructure::backend::BackendClient::device_pubkey(db),
    };
    let raw = serde_json::to_vec(&req)
        .map_err(|e| SyncError::Pairing(format!("encode pair request: {e}")))?;
    chan.send(&raw)?;
    let raw = chan.recv()?;
    let ok: PairOk = serde_json::from_slice(&raw)
        .map_err(|e| SyncError::Pairing(format!("bad pair_ok: {e}")))?;
    if ok.kind != "pair_ok" || ok.device_id != payload.device_id {
        return Err(SyncError::Pairing(
            "peer identity does not match the pairing code".into(),
        ));
    }
    bind_peer(db, &payload.device_id, &payload.static_pubkey, &payload.psk)?;
    super::engine::seed_peer_endpoint(db, &payload.device_id, &payload.addr);
    // The joiner adopts the inviter's account by redeeming the token over its own backend session.
    if let Some(token) = ok.join_token {
        redeem_join_token(db, &token);
    }
    Ok(payload.device_id)
}
