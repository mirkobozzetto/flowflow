use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use serde::Deserialize;

pub const DEFAULT_REMINDER_HOUR: u32 = 9;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReminderIntent {
    pub action: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub recurrence: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
}

impl ReminderIntent {
    pub fn resolved_date(&self) -> Option<NaiveDate> {
        let raw = self.date.as_deref()?.trim();
        if raw.is_empty() || raw.eq_ignore_ascii_case("null") {
            return None;
        }
        NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()
    }

    pub fn resolved_time(&self) -> NaiveTime {
        self.time
            .as_deref()
            .and_then(parse_hm)
            .unwrap_or_else(default_time)
    }

    pub fn has_explicit_time(&self) -> bool {
        self.time.as_deref().and_then(parse_hm).is_some()
    }

    pub fn has_date(&self) -> bool {
        self.resolved_date().is_some()
    }

    pub fn intent_hash(&self) -> String {
        let date = self
            .resolved_date()
            .map(|d| d.to_string())
            .unwrap_or_default();
        let time = self.resolved_time();
        let rec = self
            .recurrence
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_uppercase();
        format!(
            "{}|{}|{:02}:{:02}|{}",
            self.action.trim().to_lowercase(),
            date,
            time.hour(),
            time.minute(),
            rec
        )
    }
}

pub fn default_time() -> NaiveTime {
    NaiveTime::from_hms_opt(DEFAULT_REMINDER_HOUR, 0, 0)
        .expect("09:00 is a valid time")
}

fn parse_hm(raw: &str) -> Option<NaiveTime> {
    let s = raw.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("null") {
        return None;
    }
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .ok()
}

pub const BACKEND_EVENTKIT: &str = "eventkit";
pub const BACKEND_USER_NOTIFICATIONS: &str = "usernotifications";
pub const REMINDER_STATE_ACTIVE: &str = "active";
pub const REMINDER_STATE_TOMBSTONE: &str = "tombstone";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteReminder {
    pub id: String,
    pub note_id: String,
    pub reminder_id: String,
    pub backend: String,
    pub intent_hash: String,
    pub due_year: Option<i32>,
    pub due_month: Option<i32>,
    pub due_day: Option<i32>,
    pub due_hour: Option<i32>,
    pub due_minute: Option<i32>,
    pub is_all_day: bool,
    pub tz_id: Option<String>,
    pub recurrence: Option<String>,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct NewNoteReminder {
    pub note_id: String,
    pub reminder_id: String,
    pub backend: String,
    pub intent_hash: String,
    pub due_year: Option<i32>,
    pub due_month: Option<i32>,
    pub due_day: Option<i32>,
    pub due_hour: Option<i32>,
    pub due_minute: Option<i32>,
    pub is_all_day: bool,
    pub tz_id: Option<String>,
    pub recurrence: Option<String>,
}

impl NewNoteReminder {
    pub fn from_intent(
        note_id: impl Into<String>,
        reminder_id: impl Into<String>,
        backend: impl Into<String>,
        intent: &ReminderIntent,
        tz_id: Option<String>,
    ) -> Self {
        let date = intent.resolved_date();
        let time = intent.resolved_time();
        Self {
            note_id: note_id.into(),
            reminder_id: reminder_id.into(),
            backend: backend.into(),
            intent_hash: intent.intent_hash(),
            due_year: date.map(|d| d.year()),
            due_month: date.map(|d| d.month() as i32),
            due_day: date.map(|d| d.day() as i32),
            due_hour: Some(time.hour() as i32),
            due_minute: Some(time.minute() as i32),
            is_all_day: false,
            tz_id,
            recurrence: intent.recurrence.clone(),
        }
    }
}
