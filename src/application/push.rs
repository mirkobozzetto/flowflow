//! Notifications on this iPhone: the APNs token goes to the FlowFlow server,
//! which alerts every device of a premium account.

use crate::infrastructure::backend::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;

pub const ENABLED_SETTING: &str = "push_enabled";
/// Time to close the app before the test alert lands.
pub const TEST_DELAY_SECS: u64 = 10;

// A `make all` build is signed with a development profile, so its token only
// works against the APNs sandbox; App Store builds are release builds.
const ENVIRONMENT: &str = if cfg!(debug_assertions) {
    "sandbox"
} else {
    "production"
};

#[derive(Clone, Debug, PartialEq)]
pub enum PushError {
    NoBackend,
    /// The person refused notifications in iOS.
    Denied,
    NotPremium,
    NoDevice,
    /// The server has no APNs key.
    Unavailable,
    Failed(String),
}

impl From<BackendError> for PushError {
    fn from(e: BackendError) -> Self {
        match &e {
            BackendError::Status(403, _) => PushError::NotPremium,
            BackendError::Status(409, body) if body.contains("no_push_device") => {
                PushError::NoDevice
            }
            BackendError::Status(409, body)
                if body.contains("push_unavailable") =>
            {
                PushError::Unavailable
            }
            _ => PushError::Failed(e.to_string()),
        }
    }
}

pub fn is_enabled(db: &Database) -> bool {
    db.get_setting(ENABLED_SETTING).as_deref() == Some("1")
}

fn client(db: &Database) -> Result<BackendClient, PushError> {
    BackendClient::from_db(db).ok_or(PushError::NoBackend)
}

/// Asks iOS for permission and a token, then hands the token to the server.
pub async fn enable(db: &Database) -> Result<(), PushError> {
    let token = device_token().await?;
    eprintln!("[push] APNs token received ({} chars)", token.len());
    register(db, &token).await
}

pub async fn register(db: &Database, token: &str) -> Result<(), PushError> {
    client(db)?.register_push(db, token, ENVIRONMENT).await?;
    db.set_setting(ENABLED_SETTING, "1")
        .map_err(PushError::Failed)
}

/// Stays on when the server could not be told: it would keep sending.
pub async fn disable(db: &Database) -> Result<(), PushError> {
    client(db)?.unregister_push(db).await?;
    db.set_setting(ENABLED_SETTING, "0")
        .map_err(PushError::Failed)
}

pub async fn send_test(db: &Database, body: &str) -> Result<(), PushError> {
    Ok(client(db)?.test_push(db, body, TEST_DELAY_SECS).await?)
}

/// iOS can hand out a new token after a restore or reinstall; Apple asks
/// apps to register at every launch.
pub async fn refresh(db: &Database) {
    if is_enabled(db) {
        if let Err(e) = enable(db).await {
            eprintln!("[push] refresh failed: {e:?}");
        }
    }
}

#[cfg(target_os = "ios")]
async fn device_token() -> Result<String, PushError> {
    use crate::infrastructure::platform::ios::push::{self, TokenError};
    push::device_token().await.map_err(|e| match e {
        TokenError::Denied => PushError::Denied,
        TokenError::Failed(reason) => PushError::Failed(reason),
    })
}

#[cfg(not(target_os = "ios"))]
async fn device_token() -> Result<String, PushError> {
    Err(PushError::Failed("notifications are iPhone only".into()))
}
