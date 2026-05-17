#[derive(Clone, PartialEq)]
pub struct ChatSource {
    pub note_id: String,
    pub title: String,
    pub chunk_text: String,
    pub distance: f32,
}

#[derive(Clone)]
pub enum ChatMsg {
    User(String),
    Bot {
        text: String,
        sources: Vec<ChatSource>,
    },
}

pub fn tool_label(name: &str) -> &str {
    match name {
        "search_notes" => "Recherche dans les notes...",
        "create_note" => "Création de la note...",
        "summarize_folder" => "Résumé du dossier...",
        _ => "L'agent travaille...",
    }
}
