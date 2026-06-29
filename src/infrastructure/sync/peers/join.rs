use super::account_join::redeem_join_token;
use super::host::{PairOk, PairRequest};
use super::peer_store::bind_peer;
use super::{decode_pairing_uri, ensure_sync_identity};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::{engine, transport, SyncError};

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
    engine::seed_peer_endpoint(db, &payload.device_id, &payload.addr);
    // The joiner adopts the inviter's account by redeeming the token over its own backend session.
    if let Some(token) = ok.join_token {
        redeem_join_token(db, &token);
    }
    Ok(payload.device_id)
}
