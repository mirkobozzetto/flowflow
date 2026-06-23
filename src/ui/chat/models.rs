#[derive(Clone, PartialEq)]
pub struct ChatSource {
    pub note_id: String,
    pub title: String,
    pub chunk_text: String,
    pub distance: f32,
    pub created_at: String,
    pub url: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ChatMsg {
    User(String),
    Bot {
        text: String,
        sources: Vec<ChatSource>,
    },
}

use crate::application::i18n::t;

pub fn tool_label(lang: &str, name: &str) -> String {
    let key = match name {
        "search_notes" => "chat-tool-search",
        "create_note" => "chat-tool-create",
        "summarize_folder" => "chat-tool-summarize",
        "web_search" => "chat-tool-web",
        _ => "chat-tool-working",
    };
    t(lang, key)
}
