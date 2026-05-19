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

const FR_MONTHS: [&str; 12] = [
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

pub fn is_auto_title(title: &str) -> bool {
    let parts: Vec<&str> = title.splitn(2, ',').collect();
    if parts.len() != 2 {
        return false;
    }
    let date_part = parts[0].trim();
    let time_part = parts[1].trim();
    if time_part.len() != 5 || !time_part.contains(':') {
        return false;
    }
    FR_MONTHS.iter().any(|m| date_part.contains(m))
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
    pub audio_file_path: Option<String>,
    pub duration_secs: Option<f64>,
}

pub struct UpdateNote {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteAudio {
    pub id: String,
    pub note_id: String,
    pub file_path: String,
    pub duration_secs: Option<f64>,
    pub transcription: Option<String>,
    pub created_at: String,
}
