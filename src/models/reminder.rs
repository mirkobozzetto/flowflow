use chrono::{NaiveDate, NaiveTime};
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
