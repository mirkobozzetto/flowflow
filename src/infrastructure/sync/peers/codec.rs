use super::PSK_LEN;
use crate::infrastructure::sync::SyncError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};

pub const PAIRING_SCHEME: &str = "flowflow://pair#";

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
