use crate::infrastructure::persistence::sync_meta::DEVICE_ID_KEY;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::transport;
use crate::infrastructure::sync::SyncError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

pub const STATIC_PRIVKEY_KEY: &str = "sync_static_privkey";
pub const STATIC_PUBKEY_KEY: &str = "sync_static_pubkey";

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
