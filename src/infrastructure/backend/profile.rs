// Device plane of the person profile (proposal 0001 T05/T13): the linked web
// user's shareable fields + avatar. 404 = cluster not linked to a web account
// (the UI shows the link path) or no photo.

use std::collections::HashMap;

use super::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ProfileField {
    pub value: String,
    pub visibility: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct DeviceProfile {
    #[serde(default)]
    pub fields: HashMap<String, ProfileField>,
    #[serde(default)]
    pub avatar_hash: Option<String>,
}

impl DeviceProfile {
    pub fn display_name(&self) -> Option<&str> {
        self.fields.get("display_name").map(|f| f.value.as_str())
    }
}

impl BackendClient {
    /// The linked web user's groups+public fields and avatar hash.
    /// 404 -> Status(404, _): the cluster has no linked web account.
    pub async fn account_profile(
        &self,
        db: &Database,
    ) -> Result<DeviceProfile, BackendError> {
        let url = format!("{}/v1/account/profile", self.base_url);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        Self::read_json(resp).await
    }

    /// The avatar bytes (JPEG, already re-encoded server-side). Fetch only
    /// when the hash moved; the caller caches the file locally.
    pub async fn account_avatar(
        &self,
        db: &Database,
    ) -> Result<Vec<u8>, BackendError> {
        let url = format!("{}/v1/account/profile/avatar", self.base_url);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BackendError::Unauthorized);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BackendError::Status(status.as_u16(), body));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| BackendError::Network(format!("avatar bytes: {e}")))
    }
}
