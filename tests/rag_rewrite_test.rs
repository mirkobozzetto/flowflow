// Pure-function tests for conversational query rewriting (issue #88 F1). No LLM call, no
// key: the seams under test (turn capping, history formatting, skip decision, output
// sanitizing) are pure fns; the live rewrite stays a device concern.

use flowflow::application::constants::CHAT_HISTORY_TURNS;
use flowflow::application::rag::{
    build_rewrite_input, format_history, recent_turns, sanitize_rewrite,
    ChatTurn,
};

fn turn(role: &str, content: &str) -> ChatTurn {
    ChatTurn {
        role: role.into(),
        content: content.into(),
    }
}

#[test]
fn recent_turns_keeps_everything_under_the_cap() {
    let history = vec![turn("user", "a"), turn("bot", "b")];
    assert_eq!(recent_turns(&history), &history[..]);
}

#[test]
fn recent_turns_caps_to_most_recent() {
    let history: Vec<ChatTurn> = (0..CHAT_HISTORY_TURNS + 4)
        .map(|i| turn("user", &format!("m{i}")))
        .collect();
    let kept = recent_turns(&history);
    assert_eq!(kept.len(), CHAT_HISTORY_TURNS);
    assert_eq!(kept[0].content, "m4");
    assert_eq!(
        kept.last().unwrap().content,
        format!("m{}", CHAT_HISTORY_TURNS + 3)
    );
}

#[test]
fn format_history_labels_roles_in_order() {
    let history = vec![
        turn("user", "qui est Jean ?"),
        turn("bot", "Jean est ton plombier."),
    ];
    let out = format_history(&history);
    assert_eq!(
        out,
        "User: qui est Jean ?\nAssistant: Jean est ton plombier."
    );
}

#[test]
fn format_history_collapses_newlines_and_truncates_long_answers() {
    let long = "mot ".repeat(400);
    let history = vec![turn("bot", &format!("ligne1\nligne2  {long}"))];
    let out = format_history(&history);
    assert!(out.starts_with("Assistant: ligne1 ligne2 mot"));
    assert!(!out.contains('\n'));
    assert!(out.chars().count() <= "Assistant: ".chars().count() + 500);
}

#[test]
fn format_history_is_char_safe_on_accents() {
    let accented = "é".repeat(600);
    let history = vec![turn("user", &accented)];
    let out = format_history(&history);
    assert_eq!(out, format!("User: {}", "é".repeat(500)));
}

#[test]
fn empty_history_skips_the_rewrite_entirely() {
    assert_eq!(build_rewrite_input(&[], "et lui ?"), None);
}

#[test]
fn rewrite_input_carries_history_and_question() {
    let history = vec![turn("user", "parle-moi de Jean")];
    let input = build_rewrite_input(&history, "et lui ?").unwrap();
    assert!(input.contains("User: parle-moi de Jean"));
    assert!(input.contains("--- Last question ---\net lui ?"));
}

#[test]
fn sanitize_falls_back_on_empty_or_blank_output() {
    assert_eq!(sanitize_rewrite("", "et lui ?"), "et lui ?");
    assert_eq!(sanitize_rewrite("  \n ", "et lui ?"), "et lui ?");
    assert_eq!(sanitize_rewrite("\"\"", "et lui ?"), "et lui ?");
}

#[test]
fn sanitize_trims_quotes_and_whitespace() {
    assert_eq!(
        sanitize_rewrite("  \"notes sur Jean\" \n", "et lui ?"),
        "notes sur Jean"
    );
}

// --- parse_prep: the merged rewrite+temporal call's JSON parser ---

use flowflow::application::rag::parse_prep;

#[test]
fn parse_prep_reads_query_and_dates() {
    let raw =
        r#"{"query":"notes budget","from":"2026-05-05","to":"2026-05-11"}"#;
    let (q, dates) = parse_prep(raw, "fallback");
    assert_eq!(q, "notes budget");
    assert_eq!(
        dates,
        Some(("2026-05-05".to_string(), "2026-05-11".to_string()))
    );
}

#[test]
fn parse_prep_null_dates_mean_no_range() {
    let raw = r#"{"query":"qui est Jean ?","from":null,"to":null}"#;
    let (q, dates) = parse_prep(raw, "et lui ?");
    assert_eq!(q, "qui est Jean ?");
    assert_eq!(dates, None);
}

#[test]
fn parse_prep_tolerates_code_fences() {
    let raw = "```json\n{\"query\":\"notes sur Jean\",\"from\":null,\"to\":null}\n```";
    let (q, dates) = parse_prep(raw, "et lui ?");
    assert_eq!(q, "notes sur Jean");
    assert_eq!(dates, None);
}

#[test]
fn parse_prep_plain_text_degrades_to_bare_rewrite() {
    let (q, dates) = parse_prep("notes sur Jean", "et lui ?");
    assert_eq!(q, "notes sur Jean");
    assert_eq!(dates, None);
}

#[test]
fn parse_prep_garbage_falls_back_to_question() {
    let (q, dates) = parse_prep("", "et lui ?");
    assert_eq!(q, "et lui ?");
    assert_eq!(dates, None);

    let (q, dates) = parse_prep(r#"{"nope":1}"#, "et lui ?");
    assert_eq!(q, "et lui ?");
    assert_eq!(dates, None);
}

#[test]
fn parse_prep_partial_dates_are_ignored() {
    let raw = r#"{"query":"notes budget","from":"2026-05-05","to":null}"#;
    let (q, dates) = parse_prep(raw, "fallback");
    assert_eq!(q, "notes budget");
    assert_eq!(dates, None, "a half range is no range");
}

#[test]
fn parse_prep_empty_query_field_falls_back() {
    let raw = r#"{"query":"","from":null,"to":null}"#;
    let (q, _) = parse_prep(raw, "et lui ?");
    assert_eq!(q, "et lui ?");
}
