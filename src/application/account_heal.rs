// Account heal (RFC 0026): premium follows the pairing trust. The sync
// session advertises plan state in its Hello; after the note phases, the
// free side asks the premium side for a join token and adopts its account.
// This module owns the RULE, the cached plan state, and the backend calls;
// the wire exchange lives in infrastructure/sync/protocol/heal.rs.

use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::persistence::Database;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::time::Duration;

pub const PREMIUM_CACHE_KEY: &str = "account_premium";
pub const DEVICES_CACHE_KEY: &str = "account_devices";
pub const CHECKED_AT_KEY: &str = "account_checked_at";
pub const HEAL_BACKOFF_KEY: &str = "account_heal_backoff_until";
pub const HEAL_EVENT_KEY: &str = "account_heal_event";
pub const HEAL_ERROR_KEY: &str = "account_heal_last_error";

// A sync session must never hang on a slow backend: the responder side of
// the exchange blocks on us while we talk HTTP (peer read timeout is 20 s).
const HEAL_HTTP_TIMEOUT: Duration = Duration::from_secs(5);
// A failing join must not retry on every debounced save.
const HEAL_BACKOFF: Duration = Duration::from_secs(600);

fn block_on_with_timeout<F, T>(fut: F) -> Result<T, String>
where
    F: std::future::Future<
        Output = Result<T, crate::infrastructure::backend::BackendError>,
    >,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    rt.block_on(async {
        tokio::time::timeout(HEAL_HTTP_TIMEOUT, fut)
            .await
            .map_err(|_| "backend timeout".to_string())?
            .map_err(|e| e.to_string())
    })
}

/// Refresh the cached plan state from GET /v1/account. Best-effort: offline
/// keeps the last known state (a premium device that never reached its
/// backend advertises free and simply cannot vouch yet).
pub fn refresh_account_cache(db: &Database) {
    let Some(client) = BackendClient::from_db(db) else {
        return;
    };
    // Push my display name first so my own row already carries it in the
    // fetched view; name push is a label update, failure is non-fatal.
    let name = crate::application::device_naming::ensure_device_name(db);
    match block_on_with_timeout(async {
        if let Err(e) = client.set_device_name(db, &name).await {
            eprintln!("[heal] name push: {e}");
        }
        client.account(db).await
    }) {
        Ok(acc) => cache_account(db, &acc),
        Err(e) => eprintln!("[heal] account refresh: {e}"),
    }
}

/// Persist the plan state of an already-fetched account (the account page
/// calls this so a premium learned there reaches the sync Hello).
pub fn cache_account(
    db: &Database,
    acc: &crate::infrastructure::backend::Account,
) {
    let _ = db.set_setting(
        PREMIUM_CACHE_KEY,
        if acc.premium { "true" } else { "false" },
    );
    let _ = db.set_setting(DEVICES_CACHE_KEY, &acc.devices.len().to_string());
    let _ = db.set_setting(
        CHECKED_AT_KEY,
        &crate::infrastructure::persistence::now_iso(),
    );
}

pub fn my_premium_cached(db: &Database) -> bool {
    db.get_setting(PREMIUM_CACHE_KEY).as_deref() == Some("true")
}

pub fn my_backend_host(db: &Database) -> Option<String> {
    BackendClient::from_db(db).map(|c| c.base_url().to_string())
}

fn backoff_active(db: &Database) -> bool {
    db.get_setting(HEAL_BACKOFF_KEY)
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(&v).ok())
        .map(|until| chrono::Utc::now() < until)
        .unwrap_or(false)
}

fn arm_backoff(db: &Database) {
    let until = chrono::Utc::now()
        + chrono::Duration::from_std(HEAL_BACKOFF).unwrap_or_default();
    let _ = db.set_setting(HEAL_BACKOFF_KEY, &until.to_rfc3339());
}

/// The heal rule: ask the peer for a join token only when it is premium,
/// I am not, we target the same backend, and the last failure has cooled.
pub fn wants_join(
    db: &Database,
    peer_premium: bool,
    peer_backend_host: Option<&str>,
) -> bool {
    if !peer_premium || my_premium_cached(db) || backoff_active(db) {
        return false;
    }
    match (my_backend_host(db), peer_backend_host) {
        (Some(mine), Some(theirs)) if mine == theirs => true,
        (Some(_), Some(_)) => {
            let _ = db.set_setting(
                HEAL_ERROR_KEY,
                "devices target different backends: premium cannot spread",
            );
            false
        }
        _ => false,
    }
}

/// Signature over the Noise handshake hash: proves the enrolled pubkey is
/// held by the peer of THIS session. Returns (pubkey_b64, sig_b64).
pub fn sign_join_request(
    db: &Database,
    handshake_hash: &[u8],
) -> Option<(String, String)> {
    BackendClient::sign_with_device_key(db, handshake_hash)
}

pub fn verify_join_request(
    pubkey_b64: &str,
    sig_b64: &str,
    handshake_hash: &[u8],
) -> bool {
    let Ok(pk_bytes) = B64.decode(pubkey_b64) else {
        return false;
    };
    let Ok(pk_arr) = <[u8; 32]>::try_from(pk_bytes.as_slice()) else {
        return false;
    };
    let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) else {
        return false;
    };
    let Ok(sig_bytes) = B64.decode(sig_b64) else {
        return false;
    };
    let Ok(sig) = Signature::from_slice(&sig_bytes) else {
        return false;
    };
    vk.verify(handshake_hash, &sig).is_ok()
}

/// Premium side: mint a join token for a verified requester. My own premium
/// is re-checked by the backend (the account comes from MY session).
pub fn mint_for(db: &Database, requester_pubkey: &str) -> Option<String> {
    if !my_premium_cached(db) {
        return None;
    }
    let client = BackendClient::from_db(db)?;
    match block_on_with_timeout(client.invite(db, requester_pubkey)) {
        Ok(token) => Some(token),
        Err(e) => {
            eprintln!("[heal] invite: {e}");
            None
        }
    }
}

/// Free side: redeem the token, refresh the cache, record the event the UI
/// shows. Failure arms the backoff and stores a visible error.
pub fn redeem(db: &Database, token: &str, peer_name: &str) {
    let Some(client) = BackendClient::from_db(db) else {
        return;
    };
    match block_on_with_timeout(client.join(db, token)) {
        Ok(()) => {
            refresh_account_cache(db);
            let _ = db.set_setting(HEAL_ERROR_KEY, "");
            let _ = db.set_setting(HEAL_BACKOFF_KEY, "");
            let _ = db.set_setting(HEAL_EVENT_KEY, peer_name);
            eprintln!("[heal] adopted the account of {peer_name}");
        }
        Err(e) => {
            let _ =
                db.set_setting(HEAL_ERROR_KEY, &format!("account join: {e}"));
            arm_backoff(db);
            eprintln!("[heal] join: {e}");
        }
    }
}
