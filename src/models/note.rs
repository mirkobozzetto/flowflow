use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoteType {
    Voice,
    Text,
}

impl NoteType {
    pub fn as_str(&self) -> &str {
        match self {
            NoteType::Voice => "voice",
            NoteType::Text => "text",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "text" => NoteType::Text,
            _ => NoteType::Voice,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub note_type: NoteType,
    pub title: Option<String>,
    pub content: String,
    pub audio_file_path: Option<String>,
    pub duration_secs: Option<f64>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub modified_at: String,
}

pub struct NewVoiceNote {
    pub title: Option<String>,
    pub content: String,
    pub audio_file_path: String,
    pub duration_secs: f64,
    pub tags: Vec<String>,
}

pub struct NewTextNote {
    pub title: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
}

pub struct UpdateNote {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
}
