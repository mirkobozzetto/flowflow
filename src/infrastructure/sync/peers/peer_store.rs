use super::PSK_LEN;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::{engine, gc, SyncError};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

const PSK_KEY_PREFIX: &str = "sync_psk_";
const REBIND_OK_PREFIX: &str = "sync_rebind_ok_";
const REBIND_AT_PREFIX: &str = "sync_rebind_at_";
const REBIND_ROTATION_WINDOW_DAYS: i64 = 7;
pub const REBIND_REFUSED_MARKER: &str = "rebind requires explicit confirmation";

fn psk_key(device_id: &str) -> String {
    format!("{PSK_KEY_PREFIX}{device_id}")
}

fn key_fingerprint(pubkey_b64: &str) -> String {
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

fn rebind_recently_rotated(db: &Database, device_id: &str) -> bool {
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
pub(super) fn bind_peer(
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
            gc::clear_peer_ack(db, device_id);
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

pub fn unpair(db: &Database, device_id: &str) -> Result<(), SyncError> {
    engine::clear_peer_endpoint(db, device_id);
    gc::clear_peer_ack(db, device_id);
    db.delete_pairing(device_id, &psk_key(device_id))
        .map_err(SyncError::Pairing)
}
