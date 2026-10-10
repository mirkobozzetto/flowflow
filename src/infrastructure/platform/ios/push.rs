use std::ffi::{c_char, CStr};
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::oneshot;

extern "C" {
    fn flowflow_push_register(reply: extern "C" fn(bool, *const c_char));
}

// APNs answers in a second or two; without a network it never answers.
const TOKEN_TIMEOUT: Duration = Duration::from_secs(30);
const DENIED: &str = "denied";

#[derive(Debug)]
pub enum TokenError {
    Denied,
    Failed(String),
}

static PENDING: Mutex<Option<oneshot::Sender<Result<String, String>>>> =
    Mutex::new(None);

extern "C" fn on_reply(ok: bool, text: *const c_char) {
    let text = unsafe { CStr::from_ptr(text) }
        .to_string_lossy()
        .into_owned();
    if let Some(tx) = PENDING.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = tx.send(if ok { Ok(text) } else { Err(text) });
    }
}

/// Asks once for permission, then for the APNs token as hex.
pub async fn device_token() -> Result<String, TokenError> {
    let (tx, rx) = oneshot::channel();
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
    unsafe { flowflow_push_register(on_reply) };
    match tokio::time::timeout(TOKEN_TIMEOUT, rx).await {
        Ok(Ok(Ok(token))) => Ok(token),
        Ok(Ok(Err(reason))) if reason == DENIED => Err(TokenError::Denied),
        Ok(Ok(Err(reason))) => Err(TokenError::Failed(reason)),
        Ok(Err(_)) => Err(TokenError::Failed("superseded".into())),
        Err(_) => Err(TokenError::Failed("no answer from APNs".into())),
    }
}
