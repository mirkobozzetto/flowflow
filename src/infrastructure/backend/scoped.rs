//! Versioned owner binding requests; no schema-1 URL/header fallback.
use super::*;

impl BackendClient {
    fn scoped_url(
        &self,
        agent: &str,
        digest: &str,
        tail: &[&str],
    ) -> Result<String, BackendError> {
        if [agent, digest]
            .into_iter()
            .chain(tail.iter().copied())
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(BackendError::Network(
                "incomplete scoped route identity".into(),
            ));
        }
        let mut url =
            reqwest::Url::parse(&format!("{}/v2/agents", self.base_url))
                .map_err(|error| BackendError::Network(error.to_string()))?;
        url.path_segments_mut()
            .map_err(|_| BackendError::Network("invalid backend URL".into()))?
            .extend([agent, "packages", digest])
            .extend(tail.iter().copied());
        Ok(url.into())
    }

    pub fn scoped_mcp_url(
        &self,
        agent: &str,
        digest: &str,
        requirement: &str,
    ) -> Result<String, BackendError> {
        self.scoped_url(agent, digest, &["connectors", requirement, "mcp"])
    }

    pub async fn put_scoped_binding(
        &self,
        db: &Database,
        agent: &str,
        digest: &str,
        requirement: &str,
        slug: &str,
        resource: &serde_json::Value,
    ) -> Result<(), BackendError> {
        let url = self.scoped_url(agent, digest, &["bindings", requirement])?;
        let body =
            serde_json::json!({"connector_slug":slug,"resource":resource});
        let response = self
            .authed(db, |client, token| {
                client.put(&url).bearer_auth(token).json(&body)
            })
            .await?;
        Self::expect_success(response).await
    }

    pub async fn delete_scoped_binding(
        &self,
        db: &Database,
        agent: &str,
        digest: &str,
        requirement: &str,
    ) -> Result<(), BackendError> {
        let url = self.scoped_url(agent, digest, &["bindings", requirement])?;
        let response = self
            .authed(db, |client, token| client.delete(&url).bearer_auth(token))
            .await?;
        Self::expect_success(response).await
    }
}
