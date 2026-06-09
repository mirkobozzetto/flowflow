use super::transport;
use super::SyncError;
use crate::db::sync_meta::DEVICE_ID_KEY;
use crate::db::Database;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use std::net::{TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const PAIRING_SCHEME: &str = "flowflow://pair#";
pub const PSK_LEN: usize = 32;
pub const STATIC_PRIVKEY_KEY: &str = "sync_static_privkey";
pub const STATIC_PUBKEY_KEY: &str = "sync_static_pubkey";
pub const PSK_KEY_PREFIX: &str = "sync_psk_";
pub const PAIRING_WINDOW: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingPayload {
    pub device_id: String,
    pub addr: String,
    pub port: u16,
    pub psk: [u8; PSK_LEN],
    pub static_pubkey: Vec<u8>,
}

pub fn generate_psk() -> Result<[u8; PSK_LEN], SyncError> {
    let mut psk = [0u8; PSK_LEN];
    getrandom::getrandom(&mut psk)
        .map_err(|e| SyncError::Pairing(format!("getrandom: {e}")))?;
    Ok(psk)
}

pub fn new_pairing_payload(
    device_id: String,
    addr: String,
    port: u16,
    static_pubkey: Vec<u8>,
) -> Result<PairingPayload, SyncError> {
    Ok(PairingPayload {
        device_id,
        addr,
        port,
        psk: generate_psk()?,
        static_pubkey,
    })
}

pub fn encode_pairing_uri(p: &PairingPayload) -> Result<String, SyncError> {
    let json = serde_json::to_vec(p)
        .map_err(|e| SyncError::Pairing(format!("encode json: {e}")))?;
    Ok(format!("{PAIRING_SCHEME}{}", URL_SAFE_NO_PAD.encode(json)))
}

pub fn decode_pairing_uri(uri: &str) -> Result<PairingPayload, SyncError> {
    let b64 = uri.strip_prefix(PAIRING_SCHEME).ok_or_else(|| {
        SyncError::Pairing("unexpected pairing scheme".into())
    })?;
    let json = URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(|e| SyncError::Pairing(format!("decode base64: {e}")))?;
    serde_json::from_slice(&json)
        .map_err(|e| SyncError::Pairing(format!("decode json: {e}")))
}

pub fn pairing_qr_svg(uri: &str) -> Result<String, SyncError> {
    let code = QrCode::new(uri.as_bytes())
        .map_err(|e| SyncError::Pairing(format!("qr encode: {e}")))?;
    Ok(code.render::<svg::Color>().min_dimensions(220, 220).build())
}

pub fn parse_manual_addr(input: &str) -> Result<(String, u16), SyncError> {
    let (host, port) = input
        .rsplit_once(':')
        .ok_or_else(|| SyncError::Pairing("expected host:port".into()))?;
    if host.is_empty() {
        return Err(SyncError::Pairing("empty host".into()));
    }
    let port: u16 = port
        .parse()
        .map_err(|_| SyncError::Pairing("invalid port".into()))?;
    Ok((host.to_string(), port))
}

#[derive(Debug, Clone)]
pub struct SyncIdentity {
    pub device_id: String,
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

pub fn ensure_sync_identity(db: &Database) -> Result<SyncIdentity, SyncError> {
    let device_id = db
        .get_setting(DEVICE_ID_KEY)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| SyncError::Pairing("device_id missing".into()))?;
    let stored_priv =
        db.get_setting(STATIC_PRIVKEY_KEY).filter(|v| !v.is_empty());
    let stored_pub =
        db.get_setting(STATIC_PUBKEY_KEY).filter(|v| !v.is_empty());
    if let (Some(priv_b64), Some(pub_b64)) = (stored_priv, stored_pub) {
        let private_key = URL_SAFE_NO_PAD
            .decode(priv_b64)
            .map_err(|e| SyncError::Pairing(format!("decode privkey: {e}")))?;
        let public_key = URL_SAFE_NO_PAD
            .decode(pub_b64)
            .map_err(|e| SyncError::Pairing(format!("decode pubkey: {e}")))?;
        return Ok(SyncIdentity {
            device_id,
            private_key,
            public_key,
        });
    }
    let kp = transport::generate_static_keypair()?;
    db.set_setting(STATIC_PRIVKEY_KEY, &URL_SAFE_NO_PAD.encode(&kp.private))
        .map_err(SyncError::Pairing)?;
    db.set_setting(STATIC_PUBKEY_KEY, &URL_SAFE_NO_PAD.encode(&kp.public))
        .map_err(SyncError::Pairing)?;
    Ok(SyncIdentity {
        device_id,
        private_key: kp.private,
        public_key: kp.public,
    })
}

fn psk_key(device_id: &str) -> String {
    format!("{PSK_KEY_PREFIX}{device_id}")
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
            return Err(SyncError::Pairing(
                "device_id already paired with a different key".into(),
            ));
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

pub fn local_lan_ip() -> Result<String, SyncError> {
    if let Ok(addr) = std::env::var("FLOWFLOW_SYNC_ADVERTISE_ADDR") {
        if !addr.is_empty() {
            return Ok(addr);
        }
    }
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| SyncError::Pairing(format!("udp bind: {e}")))?;
    socket
        .connect("8.8.8.8:80")
        .map_err(|e| SyncError::Pairing(format!("no LAN route: {e}")))?;
    let addr = socket
        .local_addr()
        .map_err(|e| SyncError::Pairing(format!("local addr: {e}")))?;
    Ok(addr.ip().to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct PairRequest {
    kind: String,
    device_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PairOk {
    kind: String,
    device_id: String,
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
    let ok = PairOk {
        kind: "pair_ok".into(),
        device_id: identity.device_id.clone(),
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
    Ok(payload.device_id)
}

pub fn unpair(db: &Database, device_id: &str) -> Result<(), SyncError> {
    super::engine::clear_peer_endpoint(db, device_id);
    db.delete_pairing(device_id, &psk_key(device_id))
        .map_err(SyncError::Pairing)
}
