use chrono::{DateTime, Local};

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
    Ok(intents.into_iter().filter(|i| i.has_date()).collect())
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
