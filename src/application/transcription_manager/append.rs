use crate::domain::UpdateNote;
use crate::infrastructure::persistence::Database;

pub fn append_transcription_to_note(db: &Database, note_id: &str, text: &str) {
    let note = match db.get_note(note_id) {
        Ok(Some(n)) => n,
        _ => return,
    };
    let new_content = if note.content.is_empty() {
        text.to_string()
    } else {
        format!("{}\n{}", note.content, text)
    };
    let _ = db.update_note(
        note_id,
        &UpdateNote {
            title: None,
            content: Some(new_content.clone()),
            tags: None,
        },
    );
    crate::application::embed::embed_note(
        note_id.to_string(),
        note.title.unwrap_or_default(),
        new_content,
        note.tags,
        note.created_at,
    );
}
