//! Claude Pro/Max sign-in, the copy-code flow of Claude Code: the user signs
//! in on claude.ai, which shows a `code#state` string to paste back here.
use crate::infrastructure::token_store::TokenStore;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

const AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
const TOKEN_BASE: &str = "https://platform.claude.com";
const TOKEN_PATH: &str = "/v1/oauth/token";
const REDIRECT_URI: &str = "https://platform.claude.com/oauth/code/callback";
const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const SCOPES: &str = "org:create_api_key user:profile user:inference \
                      user:sessions:claude_code user:mcp_servers user:file_upload";
const TOKEN_EXPIRY_SKEW_SECS: i64 = 300;
const STORE: TokenStore = TokenStore {
    service: "com.flowflow.claude",
    account: "oauth",
    path_env: "FLOWFLOW_CLAUDE_AUTH_PATH",
    desktop_file: "claude_auth.json",
};

// Refresh tokens rotate: two calls refreshing at once would spend the same
// one and the loser would sign the user out.
static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone, Debug, PartialEq)]
pub struct ClaudeLogin {
    pub authorize_url: String,
    verifier: String,
}

#[derive(Debug, PartialEq)]
pub enum AccessError {
    NotConnected,
    Failed(String),
}

#[derive(Deserialize, Serialize)]
struct TokenRecord {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<i64>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

pub fn begin_login() -> Result<ClaudeLogin, String> {
    let mut bytes = [0_u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|error| error.to_string())?;
    let verifier = BASE64_URL_SAFE_NO_PAD.encode(bytes);
    let challenge =
        BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let authorize_url = url::Url::parse_with_params(
        AUTHORIZE_URL,
        [
            ("code", "true"),
            ("client_id", CLIENT_ID),
            ("response_type", "code"),
            ("redirect_uri", REDIRECT_URI),
            ("scope", SCOPES),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", verifier.as_str()),
        ],
    )
    .map_err(|error| error.to_string())?
    .to_string();
    Ok(ClaudeLogin {
        authorize_url,
        verifier,
    })
}

/// Trades the pasted `code#state` for tokens kept in the keychain.
pub async fn complete_login(
    login: &ClaudeLogin,
    pasted: &str,
) -> Result<(), String> {
    let (code, state) = match pasted.trim().split_once('#') {
        Some((code, state)) => (code.trim(), state.trim()),
        None => (pasted.trim(), login.verifier.as_str()),
    };
    if code.is_empty() {
        return Err("Missing authorization code".into());
    }
    if state != login.verifier {
        return Err("Authorization state mismatch".into());
    }
    let response = post_token(serde_json::json!({
        "grant_type": "authorization_code",
        "client_id": CLIENT_ID,
        "code": code,
        "state": state,
        "redirect_uri": REDIRECT_URI,
        "code_verifier": login.verifier,
    }))
    .await?;
    if !response.status().is_success() {
        return Err(failure("Claude token exchange", response).await);
    }
    let tokens = response
        .json::<TokenResponse>()
        .await
        .map_err(|error| error.to_string())?;
    STORE.save(&record(tokens, None))
}

/// A valid access token, refreshed when expired. A refused refresh forgets
/// the sign-in, so the user is asked to connect again.
pub async fn access_token() -> Result<String, AccessError> {
    let _refresh = REFRESH_LOCK.lock().await;
    let current = STORE
        .load::<TokenRecord>()
        .map_err(AccessError::Failed)?
        .ok_or(AccessError::NotConnected)?;
    if !expired(current.expires_at) {
        return Ok(current.access_token);
    }
    let refresh_token =
        current.refresh_token.ok_or(AccessError::NotConnected)?;
    let response = post_token(serde_json::json!({
        "grant_type": "refresh_token",
        "client_id": CLIENT_ID,
        "refresh_token": refresh_token,
    }))
    .await
    .map_err(AccessError::Failed)?;
    if matches!(response.status().as_u16(), 400 | 401) {
        let reason = failure("Claude token refresh", response).await;
        eprintln!("[claude] refresh refused, signing out: {reason}");
        STORE.delete();
        return Err(AccessError::NotConnected);
    }
    if !response.status().is_success() {
        return Err(AccessError::Failed(
            failure("Claude token refresh", response).await,
        ));
    }
    let tokens = response
        .json::<TokenResponse>()
        .await
        .map_err(|error| AccessError::Failed(error.to_string()))?;
    let refreshed = record(tokens, Some(refresh_token));
    STORE.save(&refreshed).map_err(AccessError::Failed)?;
    eprintln!("[claude] token refreshed");
    Ok(refreshed.access_token)
}

pub fn is_connected() -> bool {
    STORE.load::<TokenRecord>().ok().flatten().is_some()
}

pub fn disconnect() {
    STORE.delete();
}

async fn post_token(
    body: serde_json::Value,
) -> Result<reqwest::Response, String> {
    let base = std::env::var("FLOWFLOW_CLAUDE_AUTH_BASE")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| TOKEN_BASE.to_owned());
    reqwest::Client::new()
        .post(format!("{}{TOKEN_PATH}", base.trim_end_matches('/')))
        .json(&body)
        .send()
        .await
        .map_err(|error| error.to_string())
}

async fn failure(context: &str, response: reqwest::Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    format!("{context}: {status} {}", body.trim())
}

fn record(
    tokens: TokenResponse,
    previous_refresh: Option<String>,
) -> TokenRecord {
    TokenRecord {
        expires_at: tokens.expires_in.map(|seconds| now() + seconds),
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token.or(previous_refresh),
    }
}

fn expired(expires_at: Option<i64>) -> bool {
    expires_at.is_none_or(|at| now() >= at - TOKEN_EXPIRY_SKEW_SECS)
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
