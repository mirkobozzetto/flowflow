//! Keeps a local transcription the user started running while iOS has the
//! app in the background (iOS 26+). Elsewhere these calls do nothing.

use crate::infrastructure::persistence::Database;
#[cfg(target_os = "ios")]
use crate::infrastructure::platform::ios::continued_task;

#[cfg(target_os = "ios")]
fn labels(db: &Database, note_id: &str, percent: u8) -> (String, String) {
    use crate::application::i18n::{t, t_args, ui_lang};
    let lang = ui_lang(db);
    let title = db
        .get_note(note_id)
        .ok()
        .flatten()
        .and_then(|n| n.title)
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| t(&lang, "note-card-untitled"));
    let subtitle = t_args(
        &lang,
        "stt-background-progress",
        &[("percent", &percent.to_string())],
    );
    (title, subtitle)
}

/// iOS only grants the task to a user action taken in the foreground: a job
/// resumed at launch gets none and pauses in the background.
pub(super) fn begin(db: &Database, note_id: &str) {
    #[cfg(target_os = "ios")]
    {
        let (title, subtitle) = labels(db, note_id, 0);
        if !continued_task::begin(&title, &subtitle) {
            eprintln!("[whisper] no background task, pauses in background");
        }
    }
    #[cfg(not(target_os = "ios"))]
    let _ = (db, note_id);
}

pub(super) fn progress(
    db: &Database,
    note_id: &str,
    done_ms: u32,
    total_ms: u32,
    percent: u8,
) {
    #[cfg(target_os = "ios")]
    {
        let (title, subtitle) = labels(db, note_id, percent);
        continued_task::progress(&title, &subtitle, done_ms, total_ms);
    }
    #[cfg(not(target_os = "ios"))]
    let _ = (db, note_id, done_ms, total_ms, percent);
}

pub(super) fn end(success: bool) {
    #[cfg(target_os = "ios")]
    continued_task::end(success);
    #[cfg(not(target_os = "ios"))]
    let _ = success;
}
