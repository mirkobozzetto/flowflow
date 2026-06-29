use crate::application::constants::DEFAULT_RAG_MAX_SOURCES;
use crate::infrastructure::persistence::Database;

pub(super) fn exa_key() -> String {
    let Ok(db) = Database::open() else {
        return String::new();
    };
    crate::application::web_search::exa_api_key(&db)
}

// The web fires only when THIS chat asked for it and an Exa key exists. The
// per-chat flag replaces the old global setting: a chat is silent on the web
// until its own toggle is on, so a folder/thread chat answers from its notes by
// default and never gets its scoped hits diluted by an unscoped web merge.
pub fn resolve_web_on(chat_web: bool, exa_key: &str) -> bool {
    chat_web && !exa_key.trim().is_empty()
}

// "@note" mentions narrow this message to exactly those notes, overriding the
// chat scope. No mentions -> the scope's note set (or None for a global chat).
pub fn resolve_allowed_ids(
    scope_ids: Option<Vec<String>>,
    mention_ids: Vec<String>,
) -> Option<Vec<String>> {
    if mention_ids.is_empty() {
        scope_ids
    } else {
        Some(mention_ids)
    }
}

pub(super) fn read_max_sources() -> usize {
    Database::open()
        .ok()
        .and_then(|d| d.get_setting("rag_max_sources"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_RAG_MAX_SOURCES)
}
