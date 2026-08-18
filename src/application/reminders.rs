use crate::domain::ReminderIntent;
use crate::infrastructure::persistence::Database;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub enum ScheduleResult {
    Created,
    Duplicate,
    AccessDenied,
    Unsupported,
    Failed(String),
}

/// `schedule` run on a thread of its own, awaited through a channel. EventKit's
/// `EKEventStore` and its completion block are not `Send` and are held across an await, so
/// `schedule`'s future cannot cross a `Send` bound - which is exactly what rig's
/// `Tool::call` requires. Agent tools go through here; the note UI, which spawns on a local
/// task, keeps calling `schedule` directly. The thread opens its own `Database` rather than
/// carrying one across, so nothing but the plain intent crosses the boundary.
pub async fn schedule_off_thread(
    note_id: String,
    intent: ReminderIntent,
) -> ScheduleResult {
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let outcome = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("runtime: {e}"))
            .and_then(|rt| {
                let db = Database::open().map(Arc::new)?;
                Ok(rt.block_on(schedule(db, note_id, intent)))
            })
            .unwrap_or_else(ScheduleResult::Failed);
        let _ = tx.send(outcome);
    });
    rx.await.unwrap_or_else(|_| {
        ScheduleResult::Failed("scheduling thread died".into())
    })
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
        use crate::domain::{NewNoteReminder, BACKEND_EVENTKIT};
        use crate::infrastructure::platform::ios::reminders::{
            create_event, EventRequest, ReminderOutcome,
        };
        use chrono::{Datelike, Timelike};
        let date = intent.resolved_date().unwrap();
        let (all_day, sh, sm, eh, em) = if intent.is_event() {
            let s = intent.resolved_time();
            let e = intent.resolved_time_end().unwrap();
            (
                false,
                s.hour() as i32,
                s.minute() as i32,
                e.hour() as i32,
                e.minute() as i32,
            )
        } else if intent.has_explicit_time() {
            let s = intent.resolved_time();
            let start_tot = s.hour() as i32 * 60 + s.minute() as i32;
            let end_tot = (start_tot + 30).min(23 * 60 + 59);
            (
                false,
                start_tot / 60,
                start_tot % 60,
                end_tot / 60,
                end_tot % 60,
            )
        } else {
            (true, 9, 0, 9, 0)
        };
        let outcome = create_event(EventRequest {
            title: crate::domain::clean_event_title(&intent.action),
            notes: intent.notes_body().unwrap_or_default(),
            recurrence: intent.effective_recurrence().map(str::to_string),
            recurrence_until: intent
                .resolved_until()
                .map(|u| (u.year(), u.month() as i32, u.day() as i32)),
            year: date.year(),
            month: date.month() as i32,
            day: date.day() as i32,
            start_hour: sh,
            start_minute: sm,
            end_hour: eh,
            end_minute: em,
            all_day,
        })
        .await;
        match outcome {
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
