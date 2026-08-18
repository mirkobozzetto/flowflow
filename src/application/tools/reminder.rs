// Scheduling a reminder from an agent run. The capability itself already exists and is
// device-proven (`application::reminders::schedule`); until now the only caller was the
// suggestion UI inside a note, so asking for a reminder in chat created nothing.
//
// The tool takes the user's SENTENCE, never a date. Resolving "jeudi" is arithmetic, and a
// model has no clock: it infers a plausible date and states it with confidence. So the
// sentence goes back through `extract_reminders`, which stamps the real current time into
// the extraction prompt and resolves the date in Rust (`domain::reminder`). One extraction
// path for both surfaces, and the model never gets to invent a day.

use crate::application::constants::{CHEAP_MODEL, REMINDER_HOST_JUDGE_PROMPT};
use crate::application::embed::embed_note;
use crate::application::reminders::{schedule_off_thread, ScheduleResult};
use crate::application::reminders_extract::extract_reminders;
use crate::application::tools::{ReminderCard, ToolEvent, ToolFailure};
use crate::domain::NewTextNote;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::vectordb::VectorStore;
use chrono::Local;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct ScheduleReminder {
    pub llm: Arc<LlmClient>,
    /// The chat's event channel, so a scheduled reminder can render as a card instead of
    /// living only inside the model's prose. Cards are the whole point: a sentence saying
    /// "reminder created" leaves nothing to open, correct or undo.
    pub events: mpsc::UnboundedSender<ToolEvent>,
}

impl ScheduleReminder {
    pub fn new(
        llm: Arc<LlmClient>,
        events: mpsc::UnboundedSender<ToolEvent>,
    ) -> Self {
        Self { llm, events }
    }

    /// The note this reminder belongs on. The closest note by hybrid search, kept only when
    /// the judge confirms it is the same subject - so "prepare my case against the CPAS"
    /// lands on the CPAS file the user already keeps, and its deadline shows up there.
    ///
    /// A cosine threshold cannot make this call on a small personal corpus (the scores
    /// cluster; `rag::rerank` learned this the hard way), hence the model judges.
    async fn host_note(&self, request: &str) -> Option<(String, String)> {
        let vector = self.llm.embed(request).await.ok()?;
        let store = VectorStore::open().await.ok()?;
        let hit = store
            .hybrid_search(request, vector, 1, None)
            .await
            .ok()?
            .into_iter()
            .next()?;
        let preview = crate::application::ai::char_prefix(&hit.chunk_text, 400);
        let user_msg = format!(
            "Reminder: {request}\n\nNote: \"{}\"\n{preview}",
            hit.title
        );
        let verdict = self
            .llm
            .chat_with_model(CHEAP_MODEL, REMINDER_HOST_JUDGE_PROMPT, &user_msg)
            .await
            .ok()?;
        verdict
            .trim()
            .to_ascii_uppercase()
            .starts_with("YES")
            .then_some((hit.note_id, hit.title))
    }
}

#[derive(Deserialize)]
pub struct ScheduleReminderArgs {
    pub request: String,
}

#[derive(Serialize)]
pub struct ScheduledReminder {
    pub title: String,
    pub date: String,
    pub time: Option<String>,
}

#[derive(Serialize)]
pub struct ScheduleReminderResult {
    pub scheduled: Vec<ScheduledReminder>,
    /// Intents already on the calendar, skipped. Named so the model says "already set"
    /// instead of silently reporting a success it did not cause.
    pub already_scheduled: Vec<String>,
    /// The note the reminders hang off. Every reminder needs one: `note_reminders.note_id`
    /// is a NOT NULL foreign key, and it is what makes the reminder visible and cancelable
    /// in the app afterwards.
    pub note_id: Option<String>,
    /// Named so the model can tell the user WHERE the reminder landed - on their existing
    /// file, or on a fresh note holding the request.
    pub note_title: Option<String>,
}

impl Tool for ScheduleReminder {
    const NAME: &'static str = "schedule_reminder";
    type Error = ToolFailure;
    type Args = ScheduleReminderArgs;
    type Output = ScheduleReminderResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Schedule a reminder or an appointment in the user's calendar. \
                Use it when the user asks to be reminded of something, or to book a slot \
                (\"remind me Thursday to call the accountant\", \"dentist Tuesday at 3pm\"). \
                Pass the part of their message that asks for it, verbatim, INCLUDING the \
                words about when - not their whole message, or unrelated sentences turn into \
                extra reminders. The date is resolved by the app against the real current \
                time: never compute a date yourself, never rewrite the timing into your own \
                words. Schedule first and report after; do not ask whether to."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "request": {
                        "type": "string",
                        "description": "The user's reminder request, verbatim, with its timing words."
                    }
                },
                "required": ["request"]
            }),
        }
    }

    async fn call(
        &self,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let intents = extract_reminders(&self.llm, &args.request, Local::now())
            .await
            .map_err(|e| ToolFailure(format!("extract: {e}")))?;
        if intents.is_empty() {
            return Err(ToolFailure(
                "no date could be resolved from that request; ask the user which day"
                    .into(),
            ));
        }

        let db = Arc::new(
            Database::open()
                .map_err(|e| ToolFailure(format!("db open: {e}")))?,
        );

        // Dedup runs BEFORE the carrier note is created, so a request already on the
        // calendar leaves nothing behind.
        let mut already_scheduled = Vec::new();
        let mut fresh = Vec::new();
        for intent in intents {
            if db.reminder_exists_by_intent_hash_anywhere(&intent.intent_hash())
            {
                already_scheduled.push(intent.action);
            } else {
                fresh.push(intent);
            }
        }
        if fresh.is_empty() {
            return Ok(ScheduleReminderResult {
                scheduled: Vec::new(),
                already_scheduled,
                note_id: None,
                note_title: None,
            });
        }

        // Prefer the note the reminder is actually about; fall back to a carrier note
        // holding the request, which is also what makes the reminder visible in the app.
        let (note_id, note_title) = match self.host_note(&args.request).await {
            Some(found) => found,
            None => {
                let note = db
                    .create_text_note(&NewTextNote {
                        title: Some(fresh[0].action.clone()),
                        content: args.request.clone(),
                        tags: Vec::new(),
                    })
                    .map_err(|e| ToolFailure(format!("create note: {e}")))?;
                embed_note(
                    note.id.clone(),
                    note.title.clone().unwrap_or_default(),
                    note.content.clone(),
                    note.tags.clone(),
                    note.created_at.clone(),
                );
                (note.id, note.title.unwrap_or_default())
            }
        };

        let mut scheduled = Vec::new();
        let mut failures = Vec::new();
        for intent in fresh {
            let title = intent.action.clone();
            let hash = intent.intent_hash();
            let date = intent
                .resolved_date()
                .map(|d| d.to_string())
                .unwrap_or_default();
            let time = intent
                .has_explicit_time()
                .then(|| intent.resolved_time().format("%H:%M").to_string());
            match schedule_off_thread(note_id.clone(), intent).await {
                ScheduleResult::Created => {
                    // The row the scheduler just wrote, found back by its intent hash:
                    // the card needs it to open the iOS event sheet and to cancel.
                    if let Some(row) = db
                        .reminders_for_note(&note_id)
                        .into_iter()
                        .find(|r| r.intent_hash == hash)
                    {
                        let _ = self.events.send(ToolEvent::ReminderScheduled(
                            ReminderCard {
                                row_id: row.id,
                                title: title.clone(),
                                note_id: note_id.clone(),
                                note_title: note_title.clone(),
                            },
                        ));
                    }
                    scheduled.push(ScheduledReminder { title, date, time })
                }
                ScheduleResult::Duplicate => already_scheduled.push(title),
                ScheduleResult::AccessDenied => failures.push(
                    "calendar access is denied; the user must allow it in iOS Settings"
                        .to_string(),
                ),
                ScheduleResult::Unsupported => {
                    failures.push("reminders only work on iOS".to_string())
                }
                ScheduleResult::Failed(e) => failures.push(e),
            }
        }

        if scheduled.is_empty() && already_scheduled.is_empty() {
            return Err(ToolFailure(failures.join("; ")));
        }
        Ok(ScheduleReminderResult {
            scheduled,
            already_scheduled,
            note_id: Some(note_id),
            note_title: Some(note_title),
        })
    }
}
