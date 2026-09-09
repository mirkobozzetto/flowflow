use super::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct PremiumRequest {
    pub status: RequestStatus,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PremiumStatus {
    Active,
    Expired,
    Inactive,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Onboarding {
    pub linked: bool,
    pub account_id: Option<String>,
    pub email_verified: Option<bool>,
    pub premium_status: PremiumStatus,
    pub premium: bool,
    pub premium_expires_at: Option<String>,
    pub request: Option<PremiumRequest>,
}

impl BackendClient {
    pub async fn onboarding(
        &self,
        db: &Database,
    ) -> Result<Onboarding, BackendError> {
        let url = format!("{}/v1/account/onboarding", self.base_url);
        let response = self
            .authed(db, |client, token| client.get(&url).bearer_auth(token))
            .await?;
        Self::read_json(response).await
    }
}
