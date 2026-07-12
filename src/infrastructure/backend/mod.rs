// Device-side client for the flowflow MCP-broker backend (RFC 0008).
// Holds an Ed25519 device identity (distinct from the x25519 sync key, which cannot
// sign), proves it to the backend via a signed-nonce challenge, and caches the opaque
// session token. The feature is dark until `backend_base_url` is configured: with no
// backend, `from_db` returns None and the agent behaves exactly as before.

use crate::infrastructure::persistence::Database;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{Signer, SigningKey};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::Mutex;

const KEY_PRIVKEY: &str = "backend_device_privkey";
const KEY_PUBKEY: &str = "backend_device_pubkey";
const KEY_SESSION: &str = "backend_session_token";
const KEY_SESSION_EXP: &str = "backend_session_expires_at";
const KEY_BASE_URL: &str = "backend_base_url";

// Refresh the session this long before it actually expires.
const REFRESH_SKEW: Duration = Duration::seconds(60);

#[derive(Debug, Clone)]
pub enum BackendError {
    Network(String),
    Unauthorized,
    Status(u16, String),
    Crypto(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::Network(e) => write!(f, "network: {e}"),
            BackendError::Unauthorized => write!(f, "unauthorized"),
            BackendError::Status(c, b) => write!(f, "backend {c}: {b}"),
            BackendError::Crypto(e) => write!(f, "crypto: {e}"),
        }
    }
}

impl std::error::Error for BackendError {}

// Accept a bare host ("api.flowflow.be") as well as a full URL: default to https when no scheme is
// given, and drop a trailing slash so the path joins stay clean.
fn normalize_base_url(raw: &str) -> String {
    let u = raw.trim().trim_end_matches('/');
    if u.is_empty() || u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else {
        format!("https://{u}")
    }
}

#[derive(Clone)]
struct CachedSession {
    token: String,
    expires_at: DateTime<Utc>,
}

pub struct BackendClient {
    base_url: String,
    signing: SigningKey,
    pubkey_b64: String,
    http: reqwest::Client,
    session: Mutex<Option<CachedSession>>,
}

#[derive(Serialize)]
struct ChallengeReq {
    device_pubkey: String,
}

#[derive(serde::Deserialize)]
struct ChallengeResp {
    nonce: String,
}

#[derive(Serialize)]
struct VerifyReq {
    device_pubkey: String,
    nonce: String,
    signature: String,
}

#[derive(serde::Deserialize)]
struct SessionResp {
    session_token: String,
    expires_at: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Connector {
    pub provider: String,
    pub name: String,
    pub connected: bool,
    #[serde(default)]
    pub scopes: String,
}

#[derive(serde::Deserialize)]
struct AuthorizeResp {
    auth_url: String,
}

#[derive(Serialize)]
struct InviteReq<'a> {
    new_device_pubkey: &'a str,
}

#[derive(serde::Deserialize)]
struct InviteResp {
    join_token: String,
}

#[derive(serde::Deserialize)]
struct LinkBeginResp {
    link_token: String,
    expires_at: String,
}

#[derive(Serialize)]
struct JoinReq<'a> {
    join_token: &'a str,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct MemberDevice {
    pub device_id: String,
    pub created_at: String,
    pub last_seen: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Account {
    pub account_id: String,
    pub premium: bool,
    pub device_cap: i64,
    pub devices: Vec<MemberDevice>,
}

// One revoked (id, version) from the backend kill switch. The device matches its pinned row against
// this at arm time and refuses a revoked agent.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Revocation {
    pub id: String,
    pub version: String,
}

// A revoked connector slug (RFC 0016): forces the local pin out even while an installed agent still
// requires it - the agent then surfaces as disarmed instead of dialing into silent 403s.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ConnectorRevocation {
    pub slug: String,
}

// One published agent this device's account is entitled to (GET /v1/agents). The backend sends
// exactly {id, display_name, alias} - the directory list displays these; any unknown field stays
// ignored so a later backend addition can't break the parse.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct AgentSummary {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub alias: String,
}

impl BackendClient {
    /// Build the client if a backend is configured, else None (feature stays dark).
    /// Fallback chain mirrors the API-key loading in `llm.rs`: DB -> env -> compile-time.
    pub fn from_db(db: &Database) -> Option<Self> {
        let base_url = db
            .get_setting(KEY_BASE_URL)
            .or_else(|| std::env::var("FLOWFLOW_BACKEND_URL").ok())
            .or_else(|| option_env!("FLOWFLOW_BACKEND_URL").map(String::from))
            .map(|u| normalize_base_url(&u))
            .filter(|u| !u.is_empty())?;

        let signing = Self::ensure_identity(db).ok()?;
        let pubkey_b64 = B64.encode(signing.verifying_key().to_bytes());

        Some(Self {
            base_url,
            signing,
            pubkey_b64,
            http: reqwest::Client::new(),
            session: Mutex::new(None),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The single fixed MCP proxy endpoint the on-device rmcp client connects to.
    pub fn mcp_url(&self) -> String {
        format!("{}/v1/mcp", self.base_url)
    }

    /// The agent-scoped MCP proxy endpoint for one connector. The caller also sends
    /// `x-agent-id`; the proxy applies that agent's governance gate. `slug` is the connector id.
    pub fn connector_mcp_url(&self, slug: &str) -> String {
        format!("{}/v1/connectors/{}/mcp", self.base_url, slug)
    }

    /// The device's base64 Ed25519 public key (its backend identity). Generates and
    /// persists the keypair on first call, so it resolves even with no backend URL set -
    /// the user reads it from Settings to be added to the premium allowlist.
    pub fn device_pubkey(db: &Database) -> Option<String> {
        Self::ensure_identity(db)
            .ok()
            .map(|sk| B64.encode(sk.verifying_key().to_bytes()))
    }

    /// Load the persisted Ed25519 key, or generate + persist one on first use.
    fn ensure_identity(db: &Database) -> Result<SigningKey, BackendError> {
        if let Some(b64) = db.get_setting(KEY_PRIVKEY).filter(|v| !v.is_empty())
        {
            let bytes = B64.decode(b64).map_err(|e| {
                BackendError::Crypto(format!("privkey b64: {e}"))
            })?;
            let seed: [u8; 32] = bytes.try_into().map_err(|_| {
                BackendError::Crypto("privkey wrong length".into())
            })?;
            return Ok(SigningKey::from_bytes(&seed));
        }

        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed)
            .map_err(|e| BackendError::Crypto(format!("rng: {e}")))?;
        let signing = SigningKey::from_bytes(&seed);
        db.set_setting(KEY_PRIVKEY, &B64.encode(seed))
            .map_err(BackendError::Crypto)?;
        db.set_setting(
            KEY_PUBKEY,
            &B64.encode(signing.verifying_key().to_bytes()),
        )
        .map_err(BackendError::Crypto)?;
        Ok(signing)
    }

    /// Return a valid session token, refreshing or re-authenticating as needed.
    /// The mutex serializes acquisition so concurrent tool calls cannot double-refresh
    /// (the backend rotates on refresh and invalidates the old token).
    pub async fn session(&self, db: &Database) -> Result<String, BackendError> {
        let mut guard = self.session.lock().await;

        if guard.is_none() {
            if let (Some(tok), Some(exp)) =
                (db.get_setting(KEY_SESSION), db.get_setting(KEY_SESSION_EXP))
            {
                if let Ok(exp) = DateTime::parse_from_rfc3339(&exp) {
                    *guard = Some(CachedSession {
                        token: tok,
                        expires_at: exp.with_timezone(&Utc),
                    });
                }
            }
        }

        let now = Utc::now();
        if let Some(s) = guard.as_ref() {
            if s.expires_at - now > REFRESH_SKEW {
                return Ok(s.token.clone());
            }
            match self.do_refresh(&s.token).await {
                Ok(new) => {
                    let token = new.token.clone();
                    self.store(db, &mut guard, new);
                    return Ok(token);
                }
                // Dead/rotated token: fall through to a fresh handshake.
                Err(BackendError::Unauthorized) => {}
                Err(e) => return Err(e),
            }
        }

        let new = self.do_authenticate().await?;
        let token = new.token.clone();
        self.store(db, &mut guard, new);
        Ok(token)
    }

    async fn do_authenticate(&self) -> Result<CachedSession, BackendError> {
        let ch: ChallengeResp = self
            .post_json(
                "/v1/auth/challenge",
                &ChallengeReq {
                    device_pubkey: self.pubkey_b64.clone(),
                },
            )
            .await?;

        // The backend verifies the signature over the nonce STRING bytes (verify_strict),
        // not the decoded nonce. Match that exactly.
        let sig = self.signing.sign(ch.nonce.as_bytes());
        let verify: SessionResp = self
            .post_json(
                "/v1/auth/verify",
                &VerifyReq {
                    device_pubkey: self.pubkey_b64.clone(),
                    nonce: ch.nonce,
                    signature: B64.encode(sig.to_bytes()),
                },
            )
            .await?;
        Self::into_session(verify)
    }

    async fn do_refresh(
        &self,
        token: &str,
    ) -> Result<CachedSession, BackendError> {
        let resp = self
            .http
            .post(format!("{}/v1/auth/refresh", self.base_url))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        let parsed: SessionResp = Self::read_json(resp).await?;
        Self::into_session(parsed)
    }

    fn store(
        &self,
        db: &Database,
        guard: &mut Option<CachedSession>,
        s: CachedSession,
    ) {
        let _ = db.set_setting(KEY_SESSION, &s.token);
        let _ = db.set_setting(KEY_SESSION_EXP, &s.expires_at.to_rfc3339());
        *guard = Some(s);
    }

    fn into_session(r: SessionResp) -> Result<CachedSession, BackendError> {
        let expires_at = DateTime::parse_from_rfc3339(&r.expires_at)
            .map_err(|e| BackendError::Crypto(format!("expires_at: {e}")))?
            .with_timezone(&Utc);
        Ok(CachedSession {
            token: r.session_token,
            expires_at,
        })
    }

    async fn post_json<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R, BackendError> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Self::read_json(resp).await
    }

    async fn read_json<R: DeserializeOwned>(
        resp: reqwest::Response,
    ) -> Result<R, BackendError> {
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BackendError::Unauthorized);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BackendError::Status(status.as_u16(), body));
        }
        resp.json::<R>()
            .await
            .map_err(|e| BackendError::Network(format!("decode: {e}")))
    }

    // The signed agent package is verified by the device from its RAW JSON bytes (the digest is taken
    // over the manifest as sent), so it is read as text, not decoded into a struct here.
    async fn read_text(
        resp: reqwest::Response,
    ) -> Result<String, BackendError> {
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BackendError::Unauthorized);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BackendError::Status(status.as_u16(), body));
        }
        resp.text()
            .await
            .map_err(|e| BackendError::Network(format!("body: {e}")))
    }
}

// Agent install: fetch the signed package to verify + pin, and the revocation list (the kill switch).
impl BackendClient {
    /// Fetch the signed package for an agent the device is entitled to. Returns the raw JSON the device
    /// verifies against the pinned admin key. 403 when the agent is unpublished, revoked, or not granted.
    pub async fn fetch_agent_package(
        &self,
        db: &Database,
        agent_id: &str,
    ) -> Result<String, BackendError> {
        let url = format!("{}/v1/agents/{}/package", self.base_url, agent_id);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        Self::read_text(resp).await
    }

    /// The global revoked (id, version) list. Not entitlement-filtered: a device must learn that an
    /// agent it already installed was pulled.
    pub async fn fetch_revocations(
        &self,
        db: &Database,
    ) -> Result<Vec<Revocation>, BackendError> {
        let url = format!("{}/v1/agents/revocations", self.base_url);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        Self::read_json(resp).await
    }

    /// Every entitled connector's signed manifest envelope (RFC 0016). One call covers install and
    /// refresh; each envelope is verified against the pinned admin key before pinning.
    pub async fn fetch_connector_manifests(
        &self,
        db: &Database,
    ) -> Result<Vec<serde_json::Value>, BackendError> {
        let url = format!("{}/v1/connectors/manifests", self.base_url);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        Self::read_json(resp).await
    }

    /// The revoked connector slugs. Unfiltered, mirror of the agents kill switch.
    pub async fn fetch_connector_revocations(
        &self,
        db: &Database,
    ) -> Result<Vec<ConnectorRevocation>, BackendError> {
        let url = format!("{}/v1/connectors/revocations", self.base_url);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        Self::read_json(resp).await
    }

    /// The published agents this device's account is entitled to (already filtered
    /// server-side by plan + grants). The device directory renders this list.
    pub async fn list_agents(
        &self,
        db: &Database,
    ) -> Result<Vec<AgentSummary>, BackendError> {
        let url = format!("{}/v1/agents", self.base_url);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        Self::read_json(resp).await
    }
}

// Connector management (premium-gated backend routes). Each call attaches the device
// session as a Bearer token and retries once with a fresh session on a 401.
impl BackendClient {
    pub async fn list_connectors(
        &self,
        db: &Database,
    ) -> Result<Vec<Connector>, BackendError> {
        let url = format!("{}/v1/connectors", self.base_url);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        Self::read_json(resp).await
    }

    /// Start an OAuth connect: returns the provider auth URL to present in-app.
    pub async fn authorize(
        &self,
        db: &Database,
        provider: &str,
    ) -> Result<String, BackendError> {
        let url =
            format!("{}/v1/connectors/{}/authorize", self.base_url, provider);
        let resp = self.authed(db, |c, t| c.post(&url).bearer_auth(t)).await?;
        let parsed: AuthorizeResp = Self::read_json(resp).await?;
        Ok(parsed.auth_url)
    }

    pub async fn disconnect(
        &self,
        db: &Database,
        provider: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/connectors/{}", self.base_url, provider);
        let resp = self
            .authed(db, |c, t| c.delete(&url).bearer_auth(t))
            .await?;
        Self::expect_success(resp).await
    }

    async fn authed<F>(
        &self,
        db: &Database,
        build: F,
    ) -> Result<reqwest::Response, BackendError>
    where
        F: Fn(&reqwest::Client, &str) -> reqwest::RequestBuilder,
    {
        let token = self.session(db).await?;
        let resp = build(&self.http, &token)
            .send()
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        if resp.status() != reqwest::StatusCode::UNAUTHORIZED {
            return Ok(resp);
        }
        let token = self.force_new_session(db).await?;
        build(&self.http, &token)
            .send()
            .await
            .map_err(|e| BackendError::Network(e.to_string()))
    }

    async fn force_new_session(
        &self,
        db: &Database,
    ) -> Result<String, BackendError> {
        let mut guard = self.session.lock().await;
        let new = self.do_authenticate().await?;
        let token = new.token.clone();
        self.store(db, &mut guard, new);
        Ok(token)
    }

    async fn expect_success(
        resp: reqwest::Response,
    ) -> Result<(), BackendError> {
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(BackendError::Unauthorized);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BackendError::Status(status.as_u16(), body));
        }
        Ok(())
    }
}

// Device-cluster account management (RFC 0009 §6ter.1). An existing member invites a new device by
// minting a server-bound, single-use join token; the new device redeems it to adopt the cluster's
// account. The account_id is never carried on the wire - only the token is.
impl BackendClient {
    /// Inviter side: mint a join token bound to the joiner's Ed25519 backend pubkey. The account is
    /// taken from this device's authenticated session, never asserted by the caller.
    pub async fn invite(
        &self,
        db: &Database,
        new_device_pubkey: &str,
    ) -> Result<String, BackendError> {
        let url = format!("{}/v1/account/invite", self.base_url);
        let body = InviteReq { new_device_pubkey };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        let parsed: InviteResp = Self::read_json(resp).await?;
        Ok(parsed.join_token)
    }

    /// Mint a single-use, short-TTL token a web user redeems to bind their identity to this device's
    /// account. The token is the device's consent; the account is taken from the authenticated session.
    /// Returns the token to display and its ISO expiry.
    pub async fn link_begin(
        &self,
        db: &Database,
    ) -> Result<(String, String), BackendError> {
        let url = format!("{}/v1/account/link/begin", self.base_url);
        let resp = self.authed(db, |c, t| c.post(&url).bearer_auth(t)).await?;
        let parsed: LinkBeginResp = Self::read_json(resp).await?;
        Ok((parsed.link_token, parsed.expires_at))
    }

    /// Joiner side: redeem the token on this device's own session to adopt the inviter's account.
    pub async fn join(
        &self,
        db: &Database,
        join_token: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/v1/account/join", self.base_url);
        let body = JoinReq { join_token };
        let resp = self
            .authed(db, |c, t| c.post(&url).bearer_auth(t).json(&body))
            .await?;
        Self::expect_success(resp).await
    }

    /// Read this device's account: id, premium status, the device cap, and the cluster members.
    pub async fn account(
        &self,
        db: &Database,
    ) -> Result<Account, BackendError> {
        let url = format!("{}/v1/account", self.base_url);
        let resp = self.authed(db, |c, t| c.get(&url).bearer_auth(t)).await?;
        Self::read_json(resp).await
    }

    /// Leave the account: the backend nulls this device's account_id (it drops to a fresh solo, free
    /// account) and purges the cluster's entitlements if it empties.
    pub async fn leave(&self, db: &Database) -> Result<(), BackendError> {
        let url = format!("{}/v1/account/leave", self.base_url);
        let resp = self.authed(db, |c, t| c.post(&url).bearer_auth(t)).await?;
        Self::expect_success(resp).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    // The backend verifies sig over the nonce STRING bytes with verify_strict.
    // Prove the device produces signatures that pass exactly that check.
    #[test]
    fn signs_nonce_string_bytes_verifiable() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let nonce = "AbCdEf0123456789+/=="; // a base64 nonce string
        let sig = sk.sign(nonce.as_bytes());
        let vk = sk.verifying_key();
        assert!(vk.verify_strict(nonce.as_bytes(), &sig).is_ok());
        assert!(vk.verify_strict(b"tampered", &sig).is_err());
    }

    #[test]
    fn normalize_base_url_defaults_to_https_and_trims() {
        assert_eq!(
            normalize_base_url("api.flowflow.be"),
            "https://api.flowflow.be"
        );
        assert_eq!(
            normalize_base_url("  api.flowflow.be/  "),
            "https://api.flowflow.be"
        );
        assert_eq!(
            normalize_base_url("https://api.flowflow.be/"),
            "https://api.flowflow.be"
        );
        assert_eq!(
            normalize_base_url("http://localhost:8099"),
            "http://localhost:8099"
        );
        assert_eq!(normalize_base_url(""), "");
    }

    #[test]
    fn pubkey_roundtrips_through_standard_base64() {
        let sk = SigningKey::from_bytes(&[9u8; 32]);
        let b64 = B64.encode(sk.verifying_key().to_bytes());
        let decoded = B64.decode(&b64).unwrap();
        assert_eq!(decoded.len(), 32);
        assert_eq!(decoded, sk.verifying_key().to_bytes());
    }

    // With no backend configured the client is None, so the chat surface mounts no
    // connector tools and the agent keeps exactly the three notes tools.
    #[test]
    fn from_db_is_none_when_no_backend_configured() {
        if std::env::var("FLOWFLOW_BACKEND_URL").is_ok() {
            return; // an ambient override would legitimately configure a backend
        }
        let dir = tempfile::tempdir().unwrap();
        let db = crate::infrastructure::persistence::Database::open_at(
            dir.path().join("flowflow.db"),
        )
        .unwrap();
        assert!(BackendClient::from_db(&db).is_none());
    }
}
