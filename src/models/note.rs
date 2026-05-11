use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};

pub fn generate_auto_title() -> String {
    let now = chrono::Local::now();
    let months = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    let month = months[now.month0() as usize];
    format!(
        "{} {} {}, {:02}:{:02}",
        now.day(),
        month,
        now.year(),
        now.hour(),
        now.minute()
    )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoteType {
    Voice,
    Text,
}

impl std::str::FromStr for NoteType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "text" => NoteType::Text,
            _ => NoteType::Voice,
        })
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
