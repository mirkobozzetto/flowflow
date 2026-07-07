use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Weekday};
use serde::Deserialize;

pub const DEFAULT_REMINDER_HOUR: u32 = 9;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReminderIntent {
    pub action: String,
    /// Sub-tasks of ONE appointment that share this intent's date and time. They land in the
    /// calendar event's notes body; an empty list means a plain single-action reminder.
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub time_end: Option<String>,
    #[serde(default)]
    pub recurrence: Option<String>,
    /// Explicit end date for a recurring intent (YYYY-MM-DD). Recurrence is honored ONLY when
    /// this is set; an open-ended "tous les mercredis" must not create a runaway series.
    #[serde(default)]
    pub until: Option<String>,
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

    pub fn resolved_time_end(&self) -> Option<NaiveTime> {
        self.time_end.as_deref().and_then(parse_hm)
    }

    pub fn is_event(&self) -> bool {
        match (
            self.time.as_deref().and_then(parse_hm),
            self.resolved_time_end(),
        ) {
            (Some(start), Some(end)) => end > start,
            _ => false,
        }
    }

    pub fn has_date(&self) -> bool {
        self.resolved_date().is_some()
    }

    pub fn resolved_until(&self) -> Option<NaiveDate> {
        let raw = self.until.as_deref()?.trim();
        if raw.is_empty() || raw.eq_ignore_ascii_case("null") {
            return None;
        }
        NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()
    }

    /// Recurrence honored only when an explicit end date was given. An open-ended recurring
    /// phrase degrades to a single reminder rather than an unbounded series.
    pub fn effective_recurrence(&self) -> Option<&str> {
        let rec = self.recurrence.as_deref()?.trim();
        if rec.is_empty() || rec.eq_ignore_ascii_case("null") {
            return None;
        }
        self.resolved_until().is_some().then_some(rec)
    }

    /// Render the grouped sub-tasks as the calendar event's notes body (one "- item" per line),
    /// or None when the reminder is a plain single action with no enumeration.
    pub fn notes_body(&self) -> Option<String> {
        let lines: Vec<String> = self
            .items
            .iter()
            .map(|i| i.trim())
            .filter(|i| !i.is_empty())
            .map(|i| format!("- {i}"))
            .collect();
        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    pub fn intent_hash(&self) -> String {
        let date = self
            .resolved_date()
            .map(|d| d.to_string())
            .unwrap_or_default();
        let time = self.resolved_time();
        let time_seg = if self.is_event() {
            let end = self.resolved_time_end().unwrap();
            format!(
                "{:02}:{:02}-{:02}:{:02}",
                time.hour(),
                time.minute(),
                end.hour(),
                end.minute()
            )
        } else {
            format!("{:02}:{:02}", time.hour(), time.minute())
        };
        let rec = self
            .recurrence
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_uppercase();
        let until = self
            .resolved_until()
            .map(|d| d.to_string())
            .unwrap_or_default();
        format!(
            "{}|{}|{}|{}|{}",
            self.action.trim().to_lowercase(),
            date,
            time_seg,
            rec,
            until
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceFreq {
    Daily,
    Weekly,
    Monthly,
}

/// Parsed view of the RRULE-like string the extractor emits. Covers exactly the subset the
/// prompt produces (DAILY, WEEKLY;BYDAY=.., MONTHLY;BYMONTHDAY=..); other iCalendar parts are
/// ignored. Pure, so the iOS layer only has to map it onto EventKit objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceSpec {
    pub freq: RecurrenceFreq,
    pub weekdays: Vec<Weekday>,
    pub month_days: Vec<u8>,
}

impl RecurrenceSpec {
    pub fn parse(rrule: &str) -> Option<RecurrenceSpec> {
        let s = rrule.trim().to_uppercase();
        if s.is_empty() {
            return None;
        }
        let mut parts = s.split(';').map(str::trim);
        let freq = match parts.next()? {
            "DAILY" => RecurrenceFreq::Daily,
            "WEEKLY" => RecurrenceFreq::Weekly,
            "MONTHLY" => RecurrenceFreq::Monthly,
            _ => return None,
        };
        let mut weekdays: Vec<Weekday> = Vec::new();
        let mut month_days: Vec<u8> = Vec::new();
        for part in parts {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            match key.trim() {
                "BYDAY" => {
                    for code in value.split(',') {
                        if let Some(w) = parse_byday(code.trim()) {
                            if !weekdays.contains(&w) {
                                weekdays.push(w);
                            }
                        }
                    }
                }
                "BYMONTHDAY" => {
                    for raw in value.split(',') {
                        if let Ok(n) = raw.trim().parse::<u8>() {
                            if (1..=31).contains(&n) && !month_days.contains(&n)
                            {
                                month_days.push(n);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Some(RecurrenceSpec {
            freq,
            weekdays,
            month_days,
        })
    }
}

fn parse_byday(code: &str) -> Option<Weekday> {
    Some(match code {
        "MO" => Weekday::Mon,
        "TU" => Weekday::Tue,
        "WE" => Weekday::Wed,
        "TH" => Weekday::Thu,
        "FR" => Weekday::Fri,
        "SA" => Weekday::Sat,
        "SU" => Weekday::Sun,
        _ => return None,
    })
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
            is_all_day: !intent.has_explicit_time() && !intent.is_event(),
            tz_id,
            recurrence: intent.recurrence.clone(),
        }
    }
}
