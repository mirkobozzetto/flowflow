use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub note_id: String,
    pub filename: String,
    pub content_text: String,
    pub imported_at: String,
}

pub struct NewAttachment {
    pub note_id: String,
    pub filename: String,
    pub content_text: String,
}
