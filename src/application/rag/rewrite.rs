use crate::application::ai::char_prefix;
use crate::application::constants::{
    CHAT_HISTORY_TURNS, CHEAP_MODEL, QUERY_REWRITE_PROMPT,
};
use crate::infrastructure::llm::LlmClient;

/// One prior conversation turn, decoupled from the UI's message model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

/// Last CHAT_HISTORY_TURNS turns; older turns drop first.
pub fn recent_turns(history: &[ChatTurn]) -> &[ChatTurn] {
    let start = history.len().saturating_sub(CHAT_HISTORY_TURNS);
    &history[start..]
}

/// Render turns as prompt lines. Each turn is whitespace-collapsed and truncated so the
/// rewrite call stays cheap no matter how verbose the bot has been.
pub fn format_history(turns: &[ChatTurn]) -> String {
    turns
        .iter()
        .map(|t| {
            let who = if t.role == "user" {
                "User"
            } else {
                "Assistant"
            };
            let flat =
                t.content.split_whitespace().collect::<Vec<_>>().join(" ");
            format!("{}: {}", who, char_prefix(&flat, 500))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// User message for the rewrite call; None when there is no history, so the first
/// message of a conversation never pays a rewrite round-trip.
pub fn build_rewrite_input(
    history: &[ChatTurn],
    question: &str,
) -> Option<String> {
    if history.is_empty() {
        return None;
    }
    Some(format!(
        "--- Conversation ---\n{}\n--- Last question ---\n{}",
        format_history(recent_turns(history)),
        question
    ))
}

/// Guard against a degenerate rewrite: empty output falls back to the raw question.
pub fn sanitize_rewrite(raw: &str, fallback: &str) -> String {
    let cleaned = raw.trim().trim_matches('"').trim();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.to_string()
    }
}

/// Condense (history + question) into a standalone retrieval query via the cheap model.
/// Any failure falls back to the raw question - the rewriter must never break the chat.
pub async fn rewrite_query(
    ai: &LlmClient,
    history: &[ChatTurn],
    question: &str,
) -> String {
    let Some(input) = build_rewrite_input(history, question) else {
        return question.to_string();
    };
    match ai
        .chat_with_model(CHEAP_MODEL, QUERY_REWRITE_PROMPT, &input)
        .await
    {
        Ok(raw) => sanitize_rewrite(&raw, question),
        Err(_) => question.to_string(),
    }
}
