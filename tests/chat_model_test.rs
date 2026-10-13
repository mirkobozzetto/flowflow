use flowflow::application::{chat_model, chat_source};
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::Provider;
use tempfile::tempdir;

#[test]
fn each_conversation_keeps_its_pick_and_new_ones_start_from_the_latest() {
    let dir = tempdir().expect("tempdir");
    // No keychain in tests: both sign-ins read empty files.
    std::env::set_var("FLOWFLOW_CHATGPT_AUTH_PATH", dir.path().join("gpt"));
    std::env::set_var("FLOWFLOW_CLAUDE_AUTH_PATH", dir.path().join("claude"));
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open test database");
    db.set_setting("anthropic_api_key", "sk-ant-test")
        .expect("claude key");
    chat_source::choose(&db, Provider::Anthropic).expect("claude source");

    let start = chat_model::current(&db, None);
    assert_eq!(
        (start.source, start.model, start.level),
        (Provider::Anthropic, "claude-haiku-5-5", "medium")
    );

    chat_model::choose_model(&db, Some("a"), Provider::Anthropic, "claude-opus-5-5")
        .expect("pick opus");
    chat_model::choose_level(&db, Some("a"), "max").expect("pick max");
    let new = chat_model::current(&db, Some("b"));
    assert_eq!((new.model, new.level), ("claude-opus-5-5", "max"));

    db.set_setting("openai_api_key", "sk-test").expect("openai key");
    let groups = chat_model::options(&db, &new);
    let offered: Vec<_> = groups
        .iter()
        .map(|(_, source, models)| {
            (*source, models.iter().map(|m| m.id).collect::<Vec<_>>())
        })
        .collect();
    assert_eq!(
        offered,
        vec![
            (
                Provider::Anthropic,
                vec!["claude-haiku-5-5", "claude-sonnet-5-5"]
            ),
            (Provider::OpenAi, vec!["gpt-6-luna", "gpt-6.1-sol"]),
        ]
    );

    let luna =
        chat_model::choose_model(&db, Some("b"), Provider::OpenAi, "gpt-6-luna")
            .expect("pick luna");
    assert_eq!((luna.model, luna.level), ("gpt-6-luna", "max"));
    assert_eq!(chat_source::active(&db), Provider::OpenAi);
    assert!(chat_model::choose_level(&db, Some("b"), "none").is_ok());

    let a = chat_model::current(&db, Some("a"));
    assert_eq!(
        (a.source, a.model, a.level),
        (Provider::Anthropic, "claude-opus-5-5", "max")
    );
    assert!(chat_model::choose_level(&db, Some("a"), "none").is_err());
}
