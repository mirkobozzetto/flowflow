use chrono::Datelike;
use serde::{Deserialize, Serialize};

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

const EN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn generate_auto_title(lang: &str) -> String {
    let now = chrono::Local::now();
    let months = if lang == "fr" { &FR_MONTHS } else { &EN_MONTHS };
    let month = months[now.month0() as usize];
    let time = if lang == "fr" {
        now.format("%H:%M").to_string()
    } else {
        now.format("%-I:%M %p").to_string()
    };
    format!("{} {} {}, {}", now.day(), month, now.year(), time)
}

pub fn is_auto_title(title: &str) -> bool {
    let parts: Vec<&str> = title.splitn(2, ',').collect();
    if parts.len() != 2 {
        return false;
    }
    let date_part = parts[0].trim();
    let time_part = parts[1].trim();
    if !time_part.contains(':') {
        return false;
    }
    let valid_time = time_part.len() == 5
        || time_part.ends_with(" AM")
        || time_part.ends_with(" PM");
    if !valid_time {
        return false;
    }
    FR_MONTHS.iter().any(|m| date_part.contains(m))
        || EN_MONTHS.iter().any(|m| date_part.contains(m))
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
    pub tags: Vec<String>,
    #[serde(default)]
    pub sources_json: Option<String>,
    #[serde(default)]
    pub thread_id: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteAudio {
    pub id: String,
    pub note_id: String,
    pub file_path: String,
    pub duration_secs: Option<f64>,
    pub transcription: Option<String>,
    pub created_at: String,
}
