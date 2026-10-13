mod support;

use flowflow::application::error::LlmError;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::{LlmClient, Provider};
use serde_json::json;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use support::{serve, Script};
use tempfile::{tempdir, TempDir};

static ENV_LOCK: Mutex<()> = Mutex::new(());

const ANSWER: &str = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet-4-6","content":[{"type":"text","text":"Bonjour"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":1}}"#;

/// A signed-in Claude subscription, no OpenAI key, Anthropic at `base`.
struct Subscribed {
    db: Database,
    _dir: TempDir,
    _lock: MutexGuard<'static, ()>,
}

impl Subscribed {
    fn at(base: &str) -> Self {
        let lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempdir().expect("tempdir");
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_secs()
            + 3600;
        let auth = dir.path().join("claude.json");
        std::fs::write(
            &auth,
            json!({
                "access_token": "token_1",
                "refresh_token": "refresh_1",
                "expires_at": expires_at,
            })
            .to_string(),
        )
        .expect("write token");
        std::env::set_var("FLOWFLOW_CLAUDE_AUTH_PATH", &auth);
        std::env::set_var("FLOWFLOW_CLAUDE_API_BASE", base);
        std::env::set_var("OPENAI_API_KEY", "");
        let db = Database::open_at(dir.path().join("flowflow_test.db"))
            .expect("open test database");
        db.set_setting("language", "en").expect("language");
        db.set_setting("ai_consent", "true").expect("consent");
        db.set_setting("llm_provider", "claude_subscription")
            .expect("provider");
        Self {
            db,
            _dir: dir,
            _lock: lock,
        }
    }
}

impl Drop for Subscribed {
    fn drop(&mut self) {
        for name in [
            "FLOWFLOW_CLAUDE_AUTH_PATH",
            "FLOWFLOW_CLAUDE_API_BASE",
            "OPENAI_API_KEY",
        ] {
            std::env::remove_var(name);
        }
    }
}

fn header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

#[test]
fn subscription_is_its_own_choice_beside_the_claude_key() {
    let provider =
        Provider::from_str("claude_subscription").expect("parse provider");
    assert_eq!(provider, Provider::ClaudeSubscription);
    assert_eq!(provider.as_str(), "claude_subscription");
    assert!(provider.is_subscription());
    assert_eq!(Provider::from_str("claude"), Ok(Provider::Anthropic));
    assert!(!Provider::Anthropic.is_subscription());
}

#[tokio::test]
async fn chat_goes_out_as_claude_code_with_the_bearer_token() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let base = serve(Script {
        responses: vec![("POST /v1/messages", support::json("200 OK", ANSWER))],
        seen: seen.clone(),
    })
    .await;
    let env = Subscribed::at(&base);
    let llm = LlmClient::from_db(&env.db).expect("no OpenAI key needed");
    assert_eq!(llm.chat_model_name(), "claude-sonnet-4-6");

    let reply = llm.chat("Answer from the notes.", "hello").await;
    assert_eq!(reply.expect("reply"), "Bonjour");

    let seen = seen.lock().unwrap();
    let request = &seen[0];
    assert_eq!(header(request, "authorization"), Some("Bearer token_1"));
    assert_eq!(header(request, "x-api-key"), None);
    assert_eq!(
        header(request, "anthropic-beta"),
        Some("claude-code-20250219,oauth-2025-04-20")
    );
    let (_, body) = request.split_once("\r\n\r\n").expect("body");
    let body: serde_json::Value = serde_json::from_str(body).expect("json");
    assert_eq!(
        body["system"][0]["text"],
        "You are Claude Code, Anthropic's official CLI for Claude."
    );
    assert_eq!(body["system"][1]["text"], "Answer from the notes.");
}

#[tokio::test]
async fn extra_usage_refusal_is_said_plainly() {
    let refusal = json!({
        "type": "error",
        "error": {
            "type": "rate_limit_error",
            "message": "Third-party apps now draw from your extra usage",
        },
    })
    .to_string();
    let base = serve(Script {
        responses: vec![(
            "POST /v1/messages",
            support::json("429 Too Many Requests", &refusal),
        )],
        seen: Arc::new(Mutex::new(Vec::new())),
    })
    .await;
    let env = Subscribed::at(&base);
    let llm = LlmClient::from_db(&env.db).expect("client");

    match llm.chat("system", "hello").await {
        Err(LlmError::NotConfigured(message)) => {
            assert!(message.starts_with("Claude refused this message"))
        }
        other => panic!("expected the extra-usage message, got {other:?}"),
    }
}

#[tokio::test]
async fn signed_out_subscription_asks_to_connect_claude() {
    let env = Subscribed::at("http://127.0.0.1:9");
    std::env::set_var(
        "FLOWFLOW_CLAUDE_AUTH_PATH",
        env._dir.path().join("missing.json"),
    );
    let llm = LlmClient::from_db(&env.db).expect("client");

    match llm.chat("system", "hello").await {
        Err(LlmError::NotConfigured(message)) => {
            assert_eq!(message, "Connect Claude in Settings > Intelligence")
        }
        other => panic!("expected the connect hint, got {other:?}"),
    }
}
