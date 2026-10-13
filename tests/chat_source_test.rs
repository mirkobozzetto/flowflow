use flowflow::application::chat_source;
use flowflow::infrastructure::persistence::Database;
use flowflow::infrastructure::Provider;
use tempfile::tempdir;

#[test]
fn one_source_answers_and_signing_out_moves_the_chat_to_a_ready_one() {
    let dir = tempdir().expect("tempdir");
    // No keychain in tests: both sign-ins read empty files.
    std::env::set_var("FLOWFLOW_CHATGPT_AUTH_PATH", dir.path().join("gpt"));
    std::env::set_var("FLOWFLOW_CLAUDE_AUTH_PATH", dir.path().join("claude"));
    let db = Database::open_at(dir.path().join("flowflow_test.db"))
        .expect("open test database");
    assert_eq!(chat_source::active(&db), Provider::OpenAi);

    db.set_setting("anthropic_api_key", "sk-ant-test")
        .expect("claude key");
    assert!(chat_source::is_ready(&db, Provider::Anthropic));
    assert!(!chat_source::is_ready(&db, Provider::ClaudeSubscription));

    chat_source::choose(&db, Provider::ClaudeSubscription).expect("choose");
    assert_eq!(chat_source::active(&db), Provider::ClaudeSubscription);
    assert!(chat_source::is_ready(&db, Provider::Anthropic));

    chat_source::fall_back_from(&db, Provider::ChatGpt).expect("no-op");
    assert_eq!(chat_source::active(&db), Provider::ClaudeSubscription);

    chat_source::fall_back_from(&db, Provider::ClaudeSubscription)
        .expect("fall back");
    assert_eq!(chat_source::active(&db), Provider::Anthropic);
}
