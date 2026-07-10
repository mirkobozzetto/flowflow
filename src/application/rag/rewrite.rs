use super::temporal::{range_from_strs, DateRange};
use crate::application::ai::char_prefix;
use crate::application::constants::{
    CHAT_HISTORY_TURNS, CHEAP_MODEL, QUERY_PREP_PROMPT,
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

/// Parse the prep model's JSON into (standalone query, optional from/to date strings).
/// Tolerates code fences; plain-text output degrades to a bare rewrite; anything
/// unusable falls back to the raw question.
pub fn parse_prep(
    raw: &str,
    fallback: &str,
) -> (String, Option<(String, String)>) {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(cleaned) else {
        return (sanitize_rewrite(raw, fallback), None);
    };
    let query = v
        .get("query")
        .and_then(|x| x.as_str())
        .map(|s| sanitize_rewrite(s, fallback))
        .unwrap_or_else(|| fallback.to_string());
    let range = match (
        v.get("from").and_then(|x| x.as_str()),
        v.get("to").and_then(|x| x.as_str()),
    ) {
        (Some(f), Some(t)) => Some((f.to_string(), t.to_string())),
        _ => None,
    };
    (query, range)
}

/// One cheap-model call turns (history + question) into a standalone retrieval query
/// AND the temporal intent - the follow-up path pays a single round-trip instead of
/// two. Any failure falls back to the raw question with no dates: prep must never
/// break the chat.
pub(super) async fn prep_query(
    ai: &LlmClient,
    history: &[ChatTurn],
    question: &str,
) -> (String, Option<DateRange>) {
    let Some(input) = build_rewrite_input(history, question) else {
        return (question.to_string(), None);
    };
    let today = chrono::Local::now().date_naive();
    let msg = format!("Today: {today}\n{input}");
    match ai
        .chat_with_model(CHEAP_MODEL, QUERY_PREP_PROMPT, &msg)
        .await
    {
        Ok(raw) => {
            let (query, dates) = parse_prep(&raw, question);
            let range = dates.and_then(|(f, t)| range_from_strs(&f, &t));
            (query, range)
        }
        Err(_) => (question.to_string(), None),
    }
}
