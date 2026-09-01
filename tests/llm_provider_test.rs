use flowflow::application::error::LlmError;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::{LlmClient, Provider};
use std::str::FromStr;
use std::sync::{Mutex, MutexGuard};
use tempfile::tempdir;

static OPENAI_ENV_LOCK: Mutex<()> = Mutex::new(());

struct OpenAiEnv {
    previous: Option<std::ffi::OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl OpenAiEnv {
    fn without_key() -> Self {
        let lock = OPENAI_ENV_LOCK.lock().expect("OpenAI env lock");
        let previous = std::env::var_os("OPENAI_API_KEY");
        std::env::set_var("OPENAI_API_KEY", "");
        Self {
            previous,
            _lock: lock,
        }
    }
}

impl Drop for OpenAiEnv {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            std::env::set_var("OPENAI_API_KEY", previous);
        } else {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }
}

fn open_test_db() -> (Database, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open test database");
    db.set_setting("language", "en").expect("set language");
    db.set_setting("ai_consent", "true")
        .expect("set AI consent");
    (db, dir)
}

#[test]
fn chatgpt_provider_aliases_round_trip() {
    for alias in ["chatgpt", "chat_gpt", "subscription"] {
        let provider =
            Provider::from_str(alias).expect("parse ChatGPT provider");
        assert_eq!(provider, Provider::ChatGpt);
        assert_eq!(provider.as_str(), "chatgpt");
    }
    assert_eq!(Provider::default(), Provider::OpenAi);
}

#[tokio::test]
async fn chatgpt_mode_allows_chat_without_openai_key_but_not_embeddings() {
    let _env = OpenAiEnv::without_key();
    let (db, _dir) = open_test_db();
    db.set_setting("llm_provider", "chatgpt")
        .expect("set ChatGPT provider");

    let client = LlmClient::from_db(&db).expect("build ChatGPT client");
    assert_eq!(client.provider(), Provider::ChatGpt);
    assert_eq!(client.chat_model_name(), "gpt-5.6-terra");

    match client.embed("test").await {
        Err(LlmError::NotConfigured(message)) => {
            assert_eq!(message, "OpenAI API key not configured")
        }
        other => panic!("expected missing OpenAI key, got {other:?}"),
    }
}

#[test]
fn openai_mode_still_requires_openai_key() {
    let _env = OpenAiEnv::without_key();
    let (db, _dir) = open_test_db();
    db.set_setting("llm_provider", "openai")
        .expect("set OpenAI provider");

    match LlmClient::from_db(&db) {
        Err(LlmError::NotConfigured(message)) => {
            assert_eq!(message, "OpenAI API key not configured")
        }
        _ => panic!("OpenAI mode must reject a missing key"),
    }
}
