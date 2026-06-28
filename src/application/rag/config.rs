use crate::application::constants::DEFAULT_RAG_MAX_SOURCES;
use crate::infrastructure::persistence::Database;

pub(super) fn web_search_config() -> (bool, String) {
    let Ok(db) = Database::open() else {
        return (false, String::new());
    };
    let enabled =
        db.get_setting("web_search_enabled").as_deref() == Some("true");
    let key = crate::application::web_search::exa_api_key(&db);
    (enabled, key)
}

pub(super) fn read_max_sources() -> usize {
    Database::open()
        .ok()
        .and_then(|d| d.get_setting("rag_max_sources"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_RAG_MAX_SOURCES)
}
