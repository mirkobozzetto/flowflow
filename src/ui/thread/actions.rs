use crate::domain::{generate_auto_title, NewThread};
use crate::infrastructure::persistence::Database;
use crate::ui::{AppState, View};
use dioxus::prelude::*;

/// Opens `thread_id`, with the note as the view to come back to.
pub fn open_thread(mut app: AppState, note_id: &str, thread_id: &str) {
    app.previous_view.set(Some(View::NoteDetail {
        note_id: note_id.to_string(),
    }));
    app.view.set(View::ThreadDetail {
        thread_id: thread_id.to_string(),
    });
}

/// Files the note into an existing thread, then opens it.
pub fn add_note_to_thread(
    mut app: AppState,
    db: &Database,
    note_id: &str,
    thread_id: &str,
) {
    let _ = db.add_note_to_thread(note_id, thread_id);
    app.notes_version.set((app.notes_version)() + 1);
    open_thread(app, note_id, thread_id);
}

/// Starts a thread named after the note, in the note's first folder, files
/// the note in it and opens it.
pub fn start_thread_with_note(
    mut app: AppState,
    db: &Database,
    lang: &str,
    note_id: &str,
) {
    let note_title = db
        .get_note(note_id)
        .ok()
        .flatten()
        .and_then(|n| n.title)
        .unwrap_or_default();
    let title = if note_title.trim().is_empty() {
        generate_auto_title(lang)
    } else {
        note_title
    };
    let folder_id = db
        .folders_for_note(note_id)
        .ok()
        .and_then(|f| f.first().map(|f| f.id.clone()));
    if let Ok(thread) = db.create_thread(&NewThread { title, folder_id }) {
        let _ = db.add_note_to_thread(note_id, &thread.id);
        app.notes_version.set((app.notes_version)() + 1);
        open_thread(app, note_id, &thread.id);
    }
}
