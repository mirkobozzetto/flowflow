use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const AUTH_BASE: &str = "https://auth.openai.com";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const VERIFY_PATH: &str = "/codex/device";
const DEVICE_CODE_PATH: &str = "/api/accounts/deviceauth/usercode";
const DEVICE_TOKEN_PATH: &str = "/api/accounts/deviceauth/token";
const OAUTH_TOKEN_PATH: &str = "/oauth/token";
const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
const DEVICE_LOGIN_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const TOKEN_EXPIRY_SKEW_SECS: i64 = 60;
const NOT_CONNECTED: &str = "ChatGPT is not connected";

#[derive(Clone, Debug)]
pub struct DeviceLogin {
    pub verify_url: String,
    pub user_code: String,
    device_auth_id: String,
    interval_secs: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TokenRecord {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_at: Option<i64>,
    account_id: Option<String>,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_auth_id: String,
    #[serde(alias = "usercode")]
    user_code: String,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    interval: Option<u64>,
}

#[derive(Deserialize)]
struct DeviceTokenResponse {
    authorization_code: String,
    code_verifier: String,
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct OAuthErrorResponse {
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn begin_device_login() -> Result<DeviceLogin, String> {
    let base = auth_base();
    let response = reqwest::Client::new()
        .post(endpoint(&base, DEVICE_CODE_PATH))
        .json(&serde_json::json!({ "client_id": CLIENT_ID }))
        .send()
        .await
        .map_err(|error| error.to_string())?;

    let response = success_response(response, "ChatGPT device login").await?;
    let device = response
        .json::<DeviceCodeResponse>()
        .await
        .map_err(|error| error.to_string())?;

    Ok(DeviceLogin {
        verify_url: endpoint(&base, VERIFY_PATH),
        user_code: device.user_code,
        device_auth_id: device.device_auth_id,
        interval_secs: device.interval.unwrap_or(DEFAULT_POLL_INTERVAL_SECS),
    })
}

pub async fn poll_device_login(login: &DeviceLogin) -> Result<(), String> {
    let client = reqwest::Client::new();
    let base = auth_base();
    let started = Instant::now();

    let code = loop {
        if started.elapsed() >= DEVICE_LOGIN_TIMEOUT {
            return Err(
                "Timed out waiting for ChatGPT device authorization".into()
            );
        }

        let response = client
            .post(endpoint(&base, DEVICE_TOKEN_PATH))
            .json(&serde_json::json!({
                "device_auth_id": login.device_auth_id,
                "user_code": login.user_code,
            }))
            .send()
            .await
            .map_err(|error| error.to_string())?;

        if response.status().is_success() {
            break response
                .json::<DeviceTokenResponse>()
                .await
                .map_err(|error| error.to_string())?;
        }

        let status = response.status();
        if status.as_u16() == 403 || status.as_u16() == 404 {
            tokio::time::sleep(Duration::from_secs(login.interval_secs)).await;
            continue;
        }

        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "ChatGPT device authorization failed: {status} {body}"
        ));
    };

    let redirect_uri = endpoint(&base, "/deviceauth/callback");
    let form = [
        ("grant_type", "authorization_code"),
        ("code", code.authorization_code.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("client_id", CLIENT_ID),
        ("code_verifier", code.code_verifier.as_str()),
    ];
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(form)
        .finish();

    let response = client
        .post(endpoint(&base, OAUTH_TOKEN_PATH))
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(body)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let response = success_response(response, "ChatGPT token exchange").await?;
    let tokens = response
        .json::<OAuthTokenResponse>()
        .await
        .map_err(|error| error.to_string())?;

    token_store::save(&build_token_record(tokens, None))
}

pub async fn access_context() -> Result<(String, Option<String>), String> {
    let mut record = token_store::load()?.ok_or_else(not_connected)?;

    if !token_expired(record.expires_at) {
        let account_id = record
            .account_id
            .clone()
            .or_else(|| extract_account_id(record.id_token.as_deref()))
            .or_else(|| extract_account_id(Some(&record.access_token)));
        if account_id != record.account_id {
            record.account_id = account_id.clone();
            token_store::save(&record)?;
        }
        return Ok((record.access_token, account_id));
    }

    let refresh_token =
        record.refresh_token.as_deref().ok_or_else(not_connected)?;
    match refresh_tokens(refresh_token).await {
        Ok(refreshed) => {
            let result =
                (refreshed.access_token.clone(), refreshed.account_id.clone());
            token_store::save(&refreshed)?;
            Ok(result)
        }
        Err(RefreshError::Reauthenticate) => {
            token_store::delete();
            Err(not_connected())
        }
        Err(RefreshError::Transport(error)) => Err(error),
    }
}

pub fn is_connected() -> bool {
    token_store::load().ok().flatten().is_some_and(|record| {
        !record.access_token.is_empty() || record.refresh_token.is_some()
    })
}

pub fn disconnect() {
    token_store::delete();
}

pub fn decode_jwt_claims(token: &str) -> serde_json::Value {
    let payload = token.split('.').nth(1).unwrap_or_default();
    BASE64_URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(serde_json::Value::Null)
}

pub fn extract_exp(token: &str) -> Option<i64> {
    decode_jwt_claims(token).get("exp").and_then(|value| {
        value.as_i64().or_else(|| value.as_u64().map(|v| v as i64))
    })
}

pub fn extract_account_id(token: Option<&str>) -> Option<String> {
    decode_jwt_claims(token?)
        .get("https://api.openai.com/auth")
        .and_then(serde_json::Value::as_object)
        .and_then(|claims| claims.get("chatgpt_account_id"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

fn auth_base() -> String {
    std::env::var("FLOWFLOW_CHATGPT_AUTH_BASE")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| AUTH_BASE.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

fn endpoint(base: &str, path: &str) -> String {
    format!("{base}{path}")
}

fn build_token_record(
    tokens: OAuthTokenResponse,
    previous_refresh_token: Option<String>,
) -> TokenRecord {
    let access_token = tokens.access_token;
    let id_token = tokens.id_token;
    let account_id = extract_account_id(id_token.as_deref())
        .or_else(|| extract_account_id(Some(&access_token)));

    TokenRecord {
        expires_at: extract_exp(&access_token),
        access_token,
        refresh_token: tokens.refresh_token.or(previous_refresh_token),
        id_token,
        account_id,
    }
}

enum RefreshError {
    Reauthenticate,
    Transport(String),
}

async fn refresh_tokens(
    refresh_token: &str,
) -> Result<TokenRecord, RefreshError> {
    let form = [
        ("client_id", CLIENT_ID),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("scope", "openid profile email"),
    ];
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(form)
        .finish();
    let response = reqwest::Client::new()
        .post(endpoint(&auth_base(), OAUTH_TOKEN_PATH))
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(body)
        .send()
        .await
        .map_err(|error| RefreshError::Transport(error.to_string()))?;

    let status = response.status();
    if status.is_success() {
        let tokens = response
            .json::<OAuthTokenResponse>()
            .await
            .map_err(|error| RefreshError::Transport(error.to_string()))?;
        return Ok(build_token_record(tokens, Some(refresh_token.to_owned())));
    }

    let body = response.text().await.unwrap_or_default();
    let oauth_error = serde_json::from_str::<OAuthErrorResponse>(&body).ok();
    if matches!(status.as_u16(), 400 | 401)
        && oauth_error
            .as_ref()
            .and_then(|error| error.error.as_deref())
            == Some("invalid_grant")
    {
        return Err(RefreshError::Reauthenticate);
    }

    Err(RefreshError::Transport(format_oauth_error(
        "ChatGPT token refresh failed",
        status,
        oauth_error.as_ref(),
        &body,
    )))
}

async fn success_response(
    response: reqwest::Response,
    context: &str,
) -> Result<reqwest::Response, String> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    let oauth_error = serde_json::from_str::<OAuthErrorResponse>(&body).ok();
    Err(format_oauth_error(
        context,
        status,
        oauth_error.as_ref(),
        &body,
    ))
}

fn format_oauth_error(
    context: &str,
    status: reqwest::StatusCode,
    oauth_error: Option<&OAuthErrorResponse>,
    body: &str,
) -> String {
    let error = oauth_error.and_then(|value| value.error.as_deref());
    let description = oauth_error
        .and_then(|value| value.error_description.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match (error, description, body.trim()) {
        (Some(error), Some(description), _) => {
            format!("{context}: {status} {error} ({description})")
        }
        (Some(error), None, _) => format!("{context}: {status} {error}"),
        (None, _, body) if !body.is_empty() => {
            format!("{context}: {status} {body}")
        }
        _ => format!("{context}: {status}"),
    }
}

fn token_expired(expires_at: Option<i64>) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();
    expires_at
        .is_none_or(|expires_at| now >= expires_at - TOKEN_EXPIRY_SKEW_SECS)
}

fn not_connected() -> String {
    NOT_CONNECTED.to_owned()
}

fn deserialize_optional_u64<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum U64OrString {
        U64(u64),
        String(String),
    }

    match Option::<U64OrString>::deserialize(deserializer)? {
        None => Ok(None),
        Some(U64OrString::U64(value)) => Ok(Some(value)),
        Some(U64OrString::String(value)) if value.trim().is_empty() => Ok(None),
        Some(U64OrString::String(value)) => value
            .trim()
            .parse()
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

mod token_store {
    use super::TokenRecord;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    const SERVICE: &str = "com.flowflow.chatgpt";
    const ACCOUNT: &str = "oauth";

    pub fn load() -> Result<Option<TokenRecord>, String> {
        match auth_path() {
            Some(path) => load_file(&path),
            None => platform_load(),
        }
    }

    pub fn save(record: &TokenRecord) -> Result<(), String> {
        match auth_path() {
            Some(path) => save_file(&path, record),
            None => platform_save(record),
        }
    }

    pub fn delete() {
        if let Some(path) = auth_path() {
            let _ = std::fs::remove_file(path);
        } else {
            platform_delete();
        }
    }

    fn auth_path() -> Option<PathBuf> {
        std::env::var_os("FLOWFLOW_CHATGPT_AUTH_PATH")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
    }

    fn load_file(
        path: &std::path::Path,
    ) -> Result<Option<TokenRecord>, String> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| error.to_string()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn save_file(
        path: &std::path::Path,
        record: &TokenRecord,
    ) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| error.to_string())?;
        }
        let bytes =
            serde_json::to_vec(record).map_err(|error| error.to_string())?;
        std::fs::write(path, bytes).map_err(|error| error.to_string())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn platform_load() -> Result<Option<TokenRecord>, String> {
        const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
        match security_framework::passwords::get_generic_password(
            SERVICE, ACCOUNT,
        ) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| error.to_string()),
            Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn platform_save(record: &TokenRecord) -> Result<(), String> {
        let bytes =
            serde_json::to_vec(record).map_err(|error| error.to_string())?;
        security_framework::passwords::set_generic_password(
            SERVICE, ACCOUNT, &bytes,
        )
        .map_err(|error| error.to_string())
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn platform_delete() {
        let _ = security_framework::passwords::delete_generic_password(
            SERVICE, ACCOUNT,
        );
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn platform_load() -> Result<Option<TokenRecord>, String> {
        load_file(&desktop_path())
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn platform_save(record: &TokenRecord) -> Result<(), String> {
        save_file(&desktop_path(), record)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn platform_delete() {
        let _ = std::fs::remove_file(desktop_path());
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn desktop_path() -> PathBuf {
        crate::infrastructure::persistence::desktop_data_dir()
            .join("chatgpt_auth.json")
    }
}
