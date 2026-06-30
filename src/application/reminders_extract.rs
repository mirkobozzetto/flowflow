use chrono::{DateTime, Local, Timelike};
use std::collections::HashMap;

use crate::application::constants::REMINDER_EXTRACTION_PROMPT;
use crate::application::error::LlmError;
use crate::domain::ReminderIntent;
use crate::infrastructure::llm::LlmClient;

pub async fn extract_reminders(
    client: &LlmClient,
    text: &str,
    now: DateTime<Local>,
) -> Result<Vec<ReminderIntent>, LlmError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let preview: String = trimmed.chars().take(4000).collect();
    let system = format!(
        "{REMINDER_EXTRACTION_PROMPT}\n\nCurrent date and time: {}.",
        now.format("%Y-%m-%d %H:%M (%A)")
    );
    let response = client.chat(&system, &preview).await?;
    let intents = parse_reminder_intents(&response)?;
    let dated: Vec<ReminderIntent> =
        intents.into_iter().filter(|i| i.has_date()).collect();
    Ok(group_same_slot(dated))
}

/// A reminder's calendar slot: same date, same start time, same recurrence = same appointment.
fn slot_key(intent: &ReminderIntent) -> String {
    let date = intent
        .resolved_date()
        .map(|d| d.to_string())
        .unwrap_or_default();
    let t = intent.resolved_time();
    let rec = intent
        .recurrence
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_uppercase();
    format!("{date}|{:02}:{:02}|{rec}", t.hour(), t.minute())
}

/// Collapse intents that fall on the SAME slot into one, so a dictation the model still split
/// task-by-task yields ONE calendar event with the rest in its notes body. The grouping decision
/// is deterministic here - the extraction prompt only suggests a nicer title, it never has to be
/// trusted for correctness. The slot's first intent keeps the title; later ones fold into `items`.
pub fn group_same_slot(intents: Vec<ReminderIntent>) -> Vec<ReminderIntent> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, ReminderIntent> = HashMap::new();
    for intent in intents {
        let key = slot_key(&intent);
        match groups.get_mut(&key) {
            Some(base) => {
                if intent.items.is_empty() {
                    base.items.push(intent.action);
                } else {
                    base.items.extend(intent.items);
                }
            }
            None => {
                order.push(key.clone());
                groups.insert(key, intent);
            }
        }
    }
    order
        .into_iter()
        .filter_map(|k| groups.remove(&k))
        .collect()
}

pub fn parse_reminder_intents(
    response: &str,
) -> Result<Vec<ReminderIntent>, LlmError> {
    #[derive(serde::Deserialize)]
    struct Envelope {
        #[serde(default)]
        intents: Vec<ReminderIntent>,
    }
    let trimmed = response.trim();
    let json = extract_json_object(trimmed).unwrap_or(trimmed);
    serde_json::from_str::<Envelope>(json)
        .map(|e| e.intents)
        .map_err(|e| {
            LlmError::ReminderParsing(format!("Invalid reminders JSON: {e}"))
        })
}

fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}
