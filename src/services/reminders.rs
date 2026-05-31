use crate::db::Database;
use crate::models::ReminderIntent;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub enum ScheduleResult {
    Created,
    Duplicate,
    AccessDenied,
    Unsupported,
    Failed(String),
}

pub async fn schedule(
    db: Arc<Database>,
    note_id: String,
    intent: ReminderIntent,
) -> ScheduleResult {
    let hash = intent.intent_hash();
    if db.reminder_exists_by_intent_hash(&note_id, &hash) {
        return ScheduleResult::Duplicate;
    }
    if intent.resolved_date().is_none() {
        return ScheduleResult::Failed("no resolvable date".into());
    }

    #[cfg(target_os = "ios")]
    {
        use crate::models::{NewNoteReminder, BACKEND_EVENTKIT};
        use crate::platform::ios::reminders::{
            create_reminder, ReminderOutcome, ReminderRequest,
        };
        use chrono::{Datelike, Timelike};
        let date = intent.resolved_date().unwrap();
        let time = intent.resolved_time();
        let req = ReminderRequest {
            title: intent.action.clone(),
            year: date.year(),
            month: date.month() as i32,
            day: date.day() as i32,
            hour: time.hour() as i32,
            minute: time.minute() as i32,
        };
        match create_reminder(req).await {
            ReminderOutcome::Created(reminder_id) => {
                let new = NewNoteReminder::from_intent(
                    note_id,
                    reminder_id,
                    BACKEND_EVENTKIT,
                    &intent,
                    None,
                );
                match db.add_note_reminder(&new) {
                    Ok(_) => ScheduleResult::Created,
                    Err(e) => ScheduleResult::Failed(e),
                }
            }
            ReminderOutcome::AccessDenied => ScheduleResult::AccessDenied,
            ReminderOutcome::Failed(e) => ScheduleResult::Failed(e),
        }
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = &intent;
        ScheduleResult::Unsupported
    }
}

pub async fn revoke_for_note(db: Arc<Database>, note_id: String) {
    for m in db.reminders_for_note(&note_id) {
        #[cfg(target_os = "ios")]
        {
            use crate::models::REMINDER_STATE_TOMBSTONE;
            match crate::platform::ios::reminders::remove_reminder(
                m.reminder_id.clone(),
            )
            .await
            {
                Ok(()) => {
                    let _ = db.delete_note_reminder(&m.id);
                }
                Err(e) => {
                    eprintln!("[reminder] revoke failed id={}: {e}", m.id);
                    let _ =
                        db.set_reminder_state(&m.id, REMINDER_STATE_TOMBSTONE);
                }
            }
        }
        #[cfg(not(target_os = "ios"))]
        {
            let _ = db.delete_note_reminder(&m.id);
        }
    }
}
