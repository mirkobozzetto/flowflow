// Device-side person profile (proposal 0001 T14): fetch the linked web
// user's shareable fields + avatar, cache the photo by content hash, and hand
// the UI a data URI (the webview displays runtime images that way - T12
// decision: the re-encoded avatar is <= ~60 KB, far under any data-URI
// ceiling). Offline or unlinked, the monogram fallback stands.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;

use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::persistence::Database;

pub const PROFILE_NAME_KEY: &str = "profile_display_name";
pub const PROFILE_AVATAR_HASH_KEY: &str = "profile_avatar_hash";
const AVATAR_FILENAME: &str = "profile_avatar.jpg";

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileStatus {
    // fields + hash refreshed (avatar re-downloaded only if the hash moved)
    Refreshed,
    // clean 404: the cluster has no linked web account (UI shows the link path)
    NotLinked,
    // no backend configured / network trouble: cached state stands
    Unavailable,
}

fn avatar_path() -> std::path::PathBuf {
    #[cfg(target_os = "ios")]
    {
        crate::infrastructure::platform::ios::documents_dir()
            .join(AVATAR_FILENAME)
    }
    #[cfg(not(target_os = "ios"))]
    {
        crate::infrastructure::persistence::desktop_data_dir()
            .join(AVATAR_FILENAME)
    }
}

/// Refresh the cached profile from the backend. Cheap when nothing moved:
/// the avatar bytes are fetched only when `avatar_hash` changed.
pub async fn refresh(db: &Database) -> ProfileStatus {
    let Some(client) = BackendClient::from_db(db) else {
        return ProfileStatus::Unavailable;
    };
    let profile = match client.account_profile(db).await {
        Ok(p) => p,
        Err(e)
            if matches!(
                e,
                crate::infrastructure::backend::BackendError::Status(404, _)
            ) =>
        {
            let _ = db.delete_setting(PROFILE_NAME_KEY);
            let _ = db.delete_setting(PROFILE_AVATAR_HASH_KEY);
            let _ = std::fs::remove_file(avatar_path());
            return ProfileStatus::NotLinked;
        }
        Err(_) => return ProfileStatus::Unavailable,
    };

    match profile.display_name() {
        Some(name) => {
            let _ = db.set_setting(PROFILE_NAME_KEY, name);
        }
        None => {
            let _ = db.delete_setting(PROFILE_NAME_KEY);
        }
    }

    let cached_hash = db.get_setting(PROFILE_AVATAR_HASH_KEY);
    match &profile.avatar_hash {
        Some(hash) if cached_hash.as_deref() != Some(hash.as_str()) => {
            if let Ok(bytes) = client.account_avatar(db).await {
                if std::fs::write(avatar_path(), &bytes).is_ok() {
                    let _ = db.set_setting(PROFILE_AVATAR_HASH_KEY, hash);
                }
            }
        }
        Some(_) => {}
        None => {
            let _ = db.delete_setting(PROFILE_AVATAR_HASH_KEY);
            let _ = std::fs::remove_file(avatar_path());
        }
    }
    ProfileStatus::Refreshed
}

/// The cached display name, if the profile carries one.
pub fn cached_display_name(db: &Database) -> Option<String> {
    db.get_setting(PROFILE_NAME_KEY).filter(|n| !n.is_empty())
}

/// The cached avatar as a data URI for the webview, or None (monogram stands).
pub fn avatar_data_uri() -> Option<String> {
    let bytes = std::fs::read(avatar_path()).ok()?;
    (!bytes.is_empty())
        .then(|| format!("data:image/jpeg;base64,{}", B64.encode(bytes)))
}
