mod account_join;
mod codec;
mod identity;
mod lan;

use super::transport;
use super::SyncError;
use crate::infrastructure::persistence::Database;
use account_join::{mint_join_token, redeem_join_token};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use lan::local_lan_ip;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use codec::{
    decode_pairing_uri, encode_pairing_uri, generate_psk, new_pairing_payload,
    pairing_qr_svg, parse_manual_addr, PairingPayload, PAIRING_SCHEME,
};
pub use identity::{
    ensure_sync_identity, SyncIdentity, STATIC_PRIVKEY_KEY, STATIC_PUBKEY_KEY,
};

pub const PSK_LEN: usize = 32;
pub const PSK_KEY_PREFIX: &str = "sync_psk_";
pub const PAIRING_WINDOW: Duration = Duration::from_secs(300);

fn psk_key(device_id: &str) -> String {
    format!("{PSK_KEY_PREFIX}{device_id}")
}

pub const REBIND_OK_PREFIX: &str = "sync_rebind_ok_";
pub const REBIND_AT_PREFIX: &str = "sync_rebind_at_";
pub const REBIND_ROTATION_WINDOW_DAYS: i64 = 7;
pub const REBIND_REFUSED_MARKER: &str = "rebind requires explicit confirmation";

pub fn key_fingerprint(pubkey_b64: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(pubkey_b64.as_bytes());
    let digest = hasher.finalize();
    digest[..4].iter().map(|b| format!("{b:02x}")).collect()
}

pub fn authorize_rebind(
    db: &Database,
    device_id: &str,
) -> Result<(), SyncError> {
    db.set_setting(&format!("{REBIND_OK_PREFIX}{device_id}"), "true")
        .map_err(SyncError::Pairing)
}

pub fn rebind_recently_rotated(db: &Database, device_id: &str) -> bool {
    let Some(at) = db
        .get_setting(&format!("{REBIND_AT_PREFIX}{device_id}"))
        .filter(|v| !v.is_empty())
    else {
        return false;
    };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&at) else {
        return false;
    };
    let age = chrono::Utc::now() - parsed.with_timezone(&chrono::Utc);
    age.num_days() < REBIND_ROTATION_WINDOW_DAYS
}

// Bind a peer's device_id to its static key, atomically with its PSK.
// Refuses an empty device_id and refuses to overwrite an existing peer whose
// stored static key differs (a peer claiming another peer's id would otherwise
// silently hijack that binding). A genuine re-pair of the same device (same
// static key) is allowed and keeps its ack/GC state.
fn bind_peer(
    db: &Database,
    device_id: &str,
    static_pubkey: &[u8],
    psk: &[u8; PSK_LEN],
) -> Result<(), SyncError> {
    if device_id.trim().is_empty() {
        return Err(SyncError::Pairing("empty peer device_id".into()));
    }
    let encoded_pub = URL_SAFE_NO_PAD.encode(static_pubkey);
    if let Some(existing) =
        db.get_peer(device_id).map_err(SyncError::Pairing)?
    {
        if existing.static_pubkey != encoded_pub {
            let ok_key = format!("{REBIND_OK_PREFIX}{device_id}");
            if db.get_setting(&ok_key).as_deref() != Some("true") {
                return Err(SyncError::Pairing(format!(
                    "device_id already paired with a different key \
                     (known {}, presented {}): {REBIND_REFUSED_MARKER}",
                    key_fingerprint(&existing.static_pubkey),
                    key_fingerprint(&encoded_pub),
                )));
            }
            let _ = db.set_setting(&ok_key, "");
            super::gc::clear_peer_ack(db, device_id);
            if rebind_recently_rotated(db, device_id) {
                eprintln!(
                    "[sync] WARNING: repeated key rotation for {device_id} - \
                     possibly a cloned identity (one backup = one lineage)"
                );
            }
            let _ = db.set_setting(
                &format!("{REBIND_AT_PREFIX}{device_id}"),
                &crate::infrastructure::persistence::now_iso(),
            );
            eprintln!(
                "[sync] confirmed rebind of {device_id}: peer row preserved, \
                 ack book cleared"
            );
        }
    }
    db.persist_pairing(
        device_id,
        &encoded_pub,
        &psk_key(device_id),
        &URL_SAFE_NO_PAD.encode(psk),
    )
    .map_err(SyncError::Pairing)
}

pub fn load_peer_psk(
    db: &Database,
    device_id: &str,
) -> Result<Option<[u8; PSK_LEN]>, SyncError> {
    let Some(b64) = db
        .get_setting(&psk_key(device_id))
        .filter(|v| !v.is_empty())
    else {
        return Ok(None);
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(|e| SyncError::Pairing(format!("decode psk: {e}")))?;
    let psk: [u8; PSK_LEN] = bytes
        .try_into()
        .map_err(|_| SyncError::Pairing("psk has wrong length".into()))?;
    Ok(Some(psk))
}

#[derive(Debug, Serialize, Deserialize)]
struct PairRequest {
    kind: String,
    device_id: String,
    // RFC 0009 Q1.2c: the joiner advertises its Ed25519 backend pubkey so the inviter can mint a
    // join token bound to it. Optional + default so a peer running pre-0009 code still parses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backend_pubkey: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PairOk {
    kind: String,
    device_id: String,
    // RFC 0009 Q1.2c: the inviter returns a server-bound join token for the joiner to redeem. The
    // raw account_id never crosses the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    join_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PairingStatus {
    Waiting,
    Paired { device_id: String },
    Failed(String),
    Cancelled,
}

#[derive(Clone)]
pub struct PairingHost {
    pub uri: String,
    pub qr_svg: String,
    pub addr: String,
    pub port: u16,
    pub status: Arc<Mutex<PairingStatus>>,
    cancel: Arc<AtomicBool>,
}

impl PairingHost {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn status(&self) -> PairingStatus {
        self.status.lock().unwrap().clone()
    }
}

pub fn start_pairing_host(db: Arc<Database>) -> Result<PairingHost, SyncError> {
    let identity = ensure_sync_identity(&db)?;
    let addr = local_lan_ip()?;
    let listener = TcpListener::bind("0.0.0.0:0")
        .map_err(|e| SyncError::Pairing(format!("listen: {e}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| SyncError::Pairing(format!("nonblocking: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| SyncError::Pairing(format!("listener addr: {e}")))?
        .port();
    let payload = new_pairing_payload(
        identity.device_id.clone(),
        addr.clone(),
        port,
        identity.public_key.clone(),
    )?;
    let uri = encode_pairing_uri(&payload)?;
    let qr_svg = pairing_qr_svg(&uri)?;
    let status = Arc::new(Mutex::new(PairingStatus::Waiting));
    let cancel = Arc::new(AtomicBool::new(false));
    let host = PairingHost {
        uri,
        qr_svg,
        addr,
        port,
        status: status.clone(),
        cancel: cancel.clone(),
    };
    let psk = payload.psk;
    std::thread::spawn(move || {
        run_pairing_host(db, listener, identity, psk, status, cancel);
    });
    Ok(host)
}

fn run_pairing_host(
    db: Arc<Database>,
    listener: TcpListener,
    identity: SyncIdentity,
    psk: [u8; PSK_LEN],
    status: Arc<Mutex<PairingStatus>>,
    cancel: Arc<AtomicBool>,
) {
    let deadline = std::time::Instant::now() + PAIRING_WINDOW;
    loop {
        if cancel.load(Ordering::SeqCst) {
            *status.lock().unwrap() = PairingStatus::Cancelled;
            return;
        }
        if std::time::Instant::now() > deadline {
            *status.lock().unwrap() =
                PairingStatus::Failed("pairing window expired".into());
            return;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if stream.set_nonblocking(false).is_err() {
                    continue;
                }
                match host_handle_connection(&db, stream, &identity, &psk) {
                    Ok(peer_device_id) => {
                        *status.lock().unwrap() = PairingStatus::Paired {
                            device_id: peer_device_id,
                        };
                        return;
                    }
                    Err(e) => {
                        *status.lock().unwrap() =
                            PairingStatus::Failed(e.to_string());
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(ref e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                // A peer that RST'd in the backlog, or a signal: keep listening
                // instead of tearing down the whole pairing window.
                continue;
            }
            Err(e) => {
                *status.lock().unwrap() =
                    PairingStatus::Failed(format!("accept: {e}"));
                return;
            }
        }
    }
}

fn host_handle_connection(
    db: &Database,
    stream: std::net::TcpStream,
    identity: &SyncIdentity,
    psk: &[u8; PSK_LEN],
) -> Result<String, SyncError> {
    let peer_ip = stream.peer_addr().ok().map(|a| a.ip().to_string());
    let mut chan =
        transport::accept_secure(stream, &identity.private_key, psk, None)?;
    let raw = chan.recv()?;
    let req: PairRequest = serde_json::from_slice(&raw)
        .map_err(|e| SyncError::Pairing(format!("bad pair request: {e}")))?;
    if req.kind != "pair_request" {
        return Err(SyncError::Pairing(format!(
            "unexpected message kind: {}",
            req.kind
        )));
    }
    if req.device_id == identity.device_id {
        return Err(SyncError::Pairing("cannot pair with self".into()));
    }
    let remote_static = chan
        .remote_static()
        .ok_or_else(|| SyncError::Pairing("peer static key missing".into()))?;
    bind_peer(db, &req.device_id, &remote_static, psk)?;
    if let Some(ip) = peer_ip {
        super::engine::seed_peer_endpoint(db, &req.device_id, &ip);
    }
    // The pairing host is the inviter: mint a join token for the joiner's backend pubkey and return
    // it in the pair_ok. Best-effort - pairing still succeeds when no backend is configured.
    let join_token = mint_join_token(db, req.backend_pubkey.as_deref());
    let ok = PairOk {
        kind: "pair_ok".into(),
        device_id: identity.device_id.clone(),
        join_token,
    };
    let raw = serde_json::to_vec(&ok)
        .map_err(|e| SyncError::Pairing(format!("encode pair_ok: {e}")))?;
    chan.send(&raw)?;
    Ok(req.device_id)
}

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

pub fn unpair(db: &Database, device_id: &str) -> Result<(), SyncError> {
    super::engine::clear_peer_endpoint(db, device_id);
    super::gc::clear_peer_ack(db, device_id);
    db.delete_pairing(device_id, &psk_key(device_id))
        .map_err(SyncError::Pairing)
}
