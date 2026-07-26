use flowflow::infrastructure::llm::{LlmClient, Provider};
use flowflow::infrastructure::persistence::Database;
use std::sync::Arc;
use tokio::sync::Mutex;

static PROVIDER_LOCK: Mutex<()> = Mutex::const_new(());

async fn with_provider<F, T>(provider: Provider, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let _guard = PROVIDER_LOCK.lock().await;
    let db = Database::open().expect("open db");
    let previous = db.get_setting("llm_provider");
    db.set_setting("llm_provider", provider.as_str())
        .expect("set provider");
    let result = f.await;
    match previous {
        Some(v) => {
            let _ = db.set_setting("llm_provider", &v);
        }
        None => {
            let _ =
                db.set_setting("llm_provider", Provider::default().as_str());
        }
    }
    result
}

#[tokio::test]
#[ignore]
async fn test_openai_provider_is_default() {
    with_provider(Provider::OpenAi, async {
        let client = LlmClient::from_env().expect("OPENAI_API_KEY required");
        assert_eq!(client.provider(), Provider::OpenAi);
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn test_anthropic_provider_construction() {
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("skip: ANTHROPIC_API_KEY not set");
        return;
    }
    with_provider(Provider::Anthropic, async {
        let client = LlmClient::from_env()
            .expect("OPENAI_API_KEY + ANTHROPIC_API_KEY required");
        assert_eq!(client.provider(), Provider::Anthropic);
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn test_anthropic_chat_real() {
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("skip: ANTHROPIC_API_KEY not set");
        return;
    }
    with_provider(Provider::Anthropic, async {
        let client = LlmClient::from_env()
            .expect("OPENAI_API_KEY + ANTHROPIC_API_KEY required");
        assert_eq!(client.provider(), Provider::Anthropic);

        let response = client
            .chat(
                "You answer in one short word only.",
                "Reply with the single word: pong",
            )
            .await
            .expect("anthropic chat should succeed");

        assert!(
            !response.trim().is_empty(),
            "anthropic response must not be empty"
        );
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn test_anthropic_agent_with_tools_real() {
    use flowflow::application::tools::prompt_agent_with_tools;

    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("skip: ANTHROPIC_API_KEY not set");
        return;
    }
    with_provider(Provider::Anthropic, async {
        let llm = Arc::new(
            LlmClient::from_env()
                .expect("OPENAI_API_KEY + ANTHROPIC_API_KEY required"),
        );
        assert_eq!(llm.provider(), Provider::Anthropic);

        let answer = prompt_agent_with_tools(
            llm,
            "Tu es un assistant qui répond brièvement en français.",
            "Bonjour, peux-tu juste dire 'pong' ?",
            None,
            flowflow::infrastructure::llm::NotesTools::Global,
        )
        .await
        .expect("anthropic agent prompt should succeed");

        assert!(
            !answer.trim().is_empty(),
            "anthropic agent response must not be empty"
        );
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn test_anthropic_missing_key_returns_error() {
    use flowflow::application::error::LlmError;

    let _guard = PROVIDER_LOCK.lock().await;
    let db = Database::open().expect("open db");
    let prev_provider = db.get_setting("llm_provider");
    let prev_key = db.get_setting("anthropic_api_key");

    db.set_setting("llm_provider", Provider::Anthropic.as_str())
        .expect("set provider");
    db.set_setting("anthropic_api_key", "")
        .expect("clear anthropic key");

    let original_env = std::env::var("ANTHROPIC_API_KEY").ok();
    std::env::remove_var("ANTHROPIC_API_KEY");

    let result = LlmClient::from_env();

    if let Some(v) = original_env {
        std::env::set_var("ANTHROPIC_API_KEY", v);
    }
    match prev_provider {
        Some(v) => {
            let _ = db.set_setting("llm_provider", &v);
        }
        None => {
            let _ =
                db.set_setting("llm_provider", Provider::default().as_str());
        }
    }
    if let Some(v) = prev_key {
        let _ = db.set_setting("anthropic_api_key", &v);
    }

    assert!(
        matches!(result, Err(LlmError::NotConfigured(_))),
        "missing anthropic key should yield NotConfigured"
    );
}
