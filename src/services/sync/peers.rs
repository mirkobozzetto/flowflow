use super::transport;
use super::SyncError;
use crate::db::sync_meta::DEVICE_ID_KEY;
use crate::db::Database;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, TcpListener, UdpSocket};
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
                &crate::db::now_iso(),
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

pub fn local_lan_ip() -> Result<String, SyncError> {
    if let Ok(addr) = std::env::var("FLOWFLOW_SYNC_ADVERTISE_ADDR") {
        if !addr.is_empty() {
            return Ok(addr);
        }
    }
    // Enumerate interfaces and pick a real Wi-Fi/LAN address first. The UDP-connect trick below
    // can latch onto a VPN/hotspot interface (utun*, e.g. 192.168.129.x) on iOS, baking an
    // unreachable address into the pairing payload, so it is only a last resort.
    if let Some(ip) = lan_ipv4_from_interfaces() {
        return Ok(ip);
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

// A usable LAN address is a private IPv4 on a physical Wi-Fi/Ethernet interface. Exclude Apple's
// VPN (utun*), cellular (pdp_ip*), AirDrop/awdl, low-latency-WLAN (llw*), bridge and hotspot (ap*)
// interfaces: those carry private IPs too (the utun 192.168.129.x bug) but are not reachable by a
// LAN peer.
fn is_usable_lan(name: &str, ip: Ipv4Addr) -> bool {
    const SKIP: &[&str] =
        &["utun", "pdp_ip", "awdl", "llw", "bridge", "ap", "lo"];
    ip.is_private() && !SKIP.iter().any(|p| name.starts_with(p))
}

// Best private IPv4 across up, non-loopback interfaces, preferring Wi-Fi (en*). Returns None when
// nothing qualifies, so the caller falls back to the UDP-route heuristic.
fn lan_ipv4_from_interfaces() -> Option<String> {
    use std::ffi::CStr;
    unsafe {
        let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifap) != 0 || ifap.is_null() {
            return None;
        }
        let mut preferred: Option<String> = None;
        let mut fallback: Option<String> = None;
        let mut cur = ifap;
        while !cur.is_null() {
            let ifa = &*cur;
            cur = ifa.ifa_next;
            if ifa.ifa_addr.is_null() || ifa.ifa_name.is_null() {
                continue;
            }
            let sa = &*ifa.ifa_addr;
            if i32::from(sa.sa_family) != libc::AF_INET {
                continue;
            }
            if ifa.ifa_flags & libc::IFF_UP as u32 == 0
                || ifa.ifa_flags & libc::IFF_LOOPBACK as u32 != 0
            {
                continue;
            }
            let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy();
            let sin = ifa.ifa_addr as *const libc::sockaddr_in;
            let o = (*sin).sin_addr.s_addr.to_ne_bytes();
            let ip = Ipv4Addr::new(o[0], o[1], o[2], o[3]);
            if !is_usable_lan(&name, ip) {
                continue;
            }
            if name.starts_with("en") {
                preferred = Some(ip.to_string());
                break;
            } else if fallback.is_none() {
                fallback = Some(ip.to_string());
            }
        }
        libc::freeifaddrs(ifap);
        preferred.or(fallback)
    }
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
    super::gc::clear_peer_ack(db, device_id);
    db.delete_pairing(device_id, &psk_key(device_id))
        .map_err(SyncError::Pairing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_lan_accepts_wifi_rejects_vpn() {
        // real Wi-Fi / Ethernet, private range
        assert!(is_usable_lan("en0", Ipv4Addr::new(192, 168, 1, 43)));
        assert!(is_usable_lan("en1", Ipv4Addr::new(10, 0, 0, 5)));
        // the bug: utun carries a private IP but is not LAN-reachable
        assert!(!is_usable_lan("utun3", Ipv4Addr::new(192, 168, 129, 5)));
        // cellular, AirDrop, loopback, public
        assert!(!is_usable_lan("pdp_ip0", Ipv4Addr::new(10, 1, 2, 3)));
        assert!(!is_usable_lan("awdl0", Ipv4Addr::new(169, 254, 1, 1)));
        assert!(!is_usable_lan("lo0", Ipv4Addr::new(127, 0, 0, 1)));
        assert!(!is_usable_lan("en0", Ipv4Addr::new(8, 8, 8, 8)));
    }
}
