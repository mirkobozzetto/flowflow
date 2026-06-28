use crate::infrastructure::persistence::Database;

// Account join over the established Noise channel (RFC 0009 §6ter.1, Q1.2c). Both sides are
// best-effort: pairing (RFC 0004 sync) must still succeed when no backend is configured or the call
// fails, so every error is logged and swallowed - the binding can be retried later from the account
// screen. Sync code runs off the async runtime (a std::thread host, a spawn_blocking joiner), so each
// call drives a short-lived runtime, matching the repo idiom (chat/actions.rs, embed.rs).
fn block_on_backend<F, T>(
    fut: F,
) -> Result<T, crate::infrastructure::backend::BackendError>
where
    F: std::future::Future<
        Output = Result<T, crate::infrastructure::backend::BackendError>,
    >,
{
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        crate::infrastructure::backend::BackendError::Network(format!(
            "runtime: {e}"
        ))
    })?;
    rt.block_on(fut)
}

// Inviter: mint a server-bound join token for the joiner's backend pubkey. None when this device has
// no backend configured, the joiner advertised no pubkey, or the mint fails.
pub(super) fn mint_join_token(
    db: &Database,
    joiner_backend_pubkey: Option<&str>,
) -> Option<String> {
    let pubkey = joiner_backend_pubkey?;
    let client = crate::infrastructure::backend::BackendClient::from_db(db)?;
    match block_on_backend(client.invite(db, pubkey)) {
        Ok(token) => Some(token),
        Err(e) => {
            eprintln!("[account] invite failed: {e}");
            None
        }
    }
}

// Joiner: redeem the token on this device's own backend session to adopt the inviter's account.
pub(super) fn redeem_join_token(db: &Database, join_token: &str) {
    let Some(client) =
        crate::infrastructure::backend::BackendClient::from_db(db)
    else {
        return;
    };
    if let Err(e) = block_on_backend(client.join(db, join_token)) {
        eprintln!("[account] join failed: {e}");
    }
}
