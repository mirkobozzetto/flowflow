mod support;

use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use flowflow::infrastructure::claude_auth::{self, AccessError};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use support::{serve, Script};
use tempfile::tempdir;

static AUTH_ENV_LOCK: Mutex<()> = Mutex::new(());

struct AuthEnv {
    _lock: MutexGuard<'static, ()>,
}

impl AuthEnv {
    fn new(path: &Path, base: &str) -> Self {
        let lock = AUTH_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::set_var("FLOWFLOW_CLAUDE_AUTH_PATH", path);
        std::env::set_var("FLOWFLOW_CLAUDE_AUTH_BASE", base);
        Self { _lock: lock }
    }
}

impl Drop for AuthEnv {
    fn drop(&mut self) {
        claude_auth::disconnect();
        std::env::remove_var("FLOWFLOW_CLAUDE_AUTH_PATH");
        std::env::remove_var("FLOWFLOW_CLAUDE_AUTH_BASE");
    }
}

fn tokens(access: &str, refresh: &str, expires_in: i64) -> String {
    support::json(
        "200 OK",
        &json!({
            "access_token": access,
            "refresh_token": refresh,
            "expires_in": expires_in,
        })
        .to_string(),
    )
}

fn state_of(login: &claude_auth::ClaudeLogin) -> String {
    url::Url::parse(&login.authorize_url)
        .expect("authorize url")
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .expect("state param")
}

fn body_of(request: &str) -> serde_json::Value {
    let (_, body) = request.split_once("\r\n\r\n").expect("request body");
    serde_json::from_str(body).expect("json body")
}

#[test]
fn login_link_carries_a_s256_challenge_of_its_state() {
    let login = claude_auth::begin_login().expect("begin login");
    let url = url::Url::parse(&login.authorize_url).expect("authorize url");
    assert_eq!(url.host_str(), Some("claude.ai"));
    assert_eq!(url.path(), "/oauth/authorize");
    let param = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
            .unwrap_or_else(|| panic!("missing {name}"))
    };
    let challenge = BASE64_URL_SAFE_NO_PAD
        .encode(Sha256::digest(param("state").as_bytes()));
    assert_eq!(param("code_challenge"), challenge);
    assert_eq!(param("code_challenge_method"), "S256");
    assert_eq!(param("response_type"), "code");
    assert!(param("scope").contains("user:inference"));
    assert_ne!(
        claude_auth::begin_login()
            .expect("second login")
            .authorize_url,
        login.authorize_url
    );
}

#[tokio::test]
async fn wrong_state_is_refused_without_calling_anthropic() {
    let dir = tempdir().expect("tempdir");
    let _env =
        AuthEnv::new(&dir.path().join("claude.json"), "http://127.0.0.1:9");
    let login = claude_auth::begin_login().expect("begin login");

    let error = claude_auth::complete_login(&login, "code_1#forged")
        .await
        .expect_err("forged state");
    assert!(error.contains("state"));
    assert!(!claude_auth::is_connected());
}

#[tokio::test]
async fn pasted_code_signs_in_then_an_expired_token_refreshes() {
    let dir = tempdir().expect("tempdir");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses: vec![
            ("POST /v1/oauth/token", tokens("access_1", "refresh_1", 0)),
            (
                "POST /v1/oauth/token",
                tokens("access_2", "refresh_2", 3600),
            ),
        ],
        seen: seen.clone(),
    })
    .await;
    let _env = AuthEnv::new(&dir.path().join("claude.json"), &base);
    let login = claude_auth::begin_login().expect("begin login");
    let state = state_of(&login);

    claude_auth::complete_login(&login, &format!(" code_1#{state} \n"))
        .await
        .expect("sign in");
    assert!(claude_auth::is_connected());

    assert_eq!(claude_auth::access_token().await.as_deref(), Ok("access_2"));
    assert_eq!(claude_auth::access_token().await.as_deref(), Ok("access_2"));

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2, "a fresh token is not refreshed again");
    let exchange = body_of(&seen[0]);
    assert_eq!(exchange["grant_type"], "authorization_code");
    assert_eq!(exchange["code"], "code_1");
    assert_eq!(exchange["state"], state.as_str());
    assert_eq!(exchange["code_verifier"], state.as_str());
    let refresh = body_of(&seen[1]);
    assert_eq!(refresh["grant_type"], "refresh_token");
    assert_eq!(refresh["refresh_token"], "refresh_1");
}

#[tokio::test]
async fn refused_refresh_signs_out() {
    let dir = tempdir().expect("tempdir");
    let base = serve(Script {
        responses: vec![
            ("POST /v1/oauth/token", tokens("access_1", "refresh_1", 0)),
            (
                "POST /v1/oauth/token",
                support::json(
                    "400 Bad Request",
                    r#"{"error":"invalid_grant"}"#,
                ),
            ),
        ],
        seen: Arc::new(Mutex::new(Vec::new())),
    })
    .await;
    let path = dir.path().join("claude.json");
    let _env = AuthEnv::new(&path, &base);
    let login = claude_auth::begin_login().expect("begin login");
    let pasted = format!("code_1#{}", state_of(&login));
    claude_auth::complete_login(&login, &pasted)
        .await
        .expect("sign in");

    assert_eq!(
        claude_auth::access_token().await,
        Err(AccessError::NotConnected)
    );
    assert!(!path.exists());
    assert!(!claude_auth::is_connected());
}

#[tokio::test]
async fn disconnect_forgets_the_token() {
    let dir = tempdir().expect("tempdir");
    let base = serve(Script {
        responses: vec![(
            "POST /v1/oauth/token",
            tokens("access_1", "refresh_1", 3600),
        )],
        seen: Arc::new(Mutex::new(Vec::new())),
    })
    .await;
    let _env = AuthEnv::new(&dir.path().join("claude.json"), &base);
    let login = claude_auth::begin_login().expect("begin login");
    let pasted = format!("code_1#{}", state_of(&login));
    claude_auth::complete_login(&login, &pasted)
        .await
        .expect("sign in");
    assert_eq!(claude_auth::access_token().await.as_deref(), Ok("access_1"));

    claude_auth::disconnect();
    assert!(!claude_auth::is_connected());
    assert_eq!(
        claude_auth::access_token().await,
        Err(AccessError::NotConnected)
    );
}
