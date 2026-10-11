use super::{BackendClient, BackendError};
use crate::infrastructure::persistence::Database;

#[derive(serde::Serialize)]
struct Registration<'a> {
    token: &'a str,
    environment: &'a str,
}

#[derive(serde::Serialize)]
struct HermesLink<'a> {
    topic: &'a str,
    token_hash: &'a str,
}

#[derive(serde::Deserialize)]
struct Linked {
    listening: bool,
}

#[derive(serde::Serialize)]
struct TestAlert<'a> {
    body: &'a str,
    delay_secs: u64,
}

impl BackendClient {
    /// Bind this device's APNs token to its account; `environment` is
    /// `sandbox` or `production`.
    pub async fn register_push(
        &self,
        db: &Database,
        token: &str,
        environment: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/push/devices", self.base_url);
        let body = Registration { token, environment };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    pub async fn unregister_push(
        &self,
        db: &Database,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/push/devices", self.base_url);
        let resp = self
            .authed(db, |c, t| c.delete(&url).bearer_auth(t))
            .await?;
        Self::expect_success(resp).await
    }

    /// Tells the server where Hermes publishes this account's task results;
    /// only the token's hash leaves the device. True when Hermes already
    /// holds that topic's stream.
    pub async fn link_hermes(
        &self,
        db: &Database,
        topic: &str,
        token_hash: &str,
    ) -> Result<bool, BackendError> {
        let url = format!("{}/v1/push/hermes-link", self.base_url);
        let body = HermesLink { topic, token_hash };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Ok(Self::read_json::<Linked>(resp).await?.listening)
    }

    /// The server sends `body` to every device of the account after
    /// `delay_secs`.
    pub async fn test_push(
        &self,
        db: &Database,
        body: &str,
        delay_secs: u64,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/push/test", self.base_url);
        let body = TestAlert { body, delay_secs };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }
}
