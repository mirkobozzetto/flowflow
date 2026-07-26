use crate::domain::{NewTextNote, Note, Transcript, UpdateNote};
use crate::infrastructure::persistence::Database;

pub fn create_note(
    db: &Database,
    title: &str,
    content: &str,
    tags: Vec<String>,
    folder_id: Option<&str>,
    audio: Option<(String, f64)>,
) -> Option<Note> {
    let new = NewTextNote {
        title: if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        },
        content: content.to_string(),
        tags,
    };
    let created = db.create_text_note(&new).ok()?;
    if let Some(fid) = folder_id {
        let _ = db.add_note_to_folder(&created.id, fid);
    }
    if let Some((path, dur)) = audio {
        let _ = db.add_audio(&created.id, &path, dur);
    }
    Some(created)
}

pub fn update_note(
    db: &Database,
    id: &str,
    title: &str,
    content: &str,
    tags: Vec<String>,
    folder_id: Option<&str>,
) -> String {
    let upd = UpdateNote {
        title: Some(title.to_string()),
        content: Some(content.to_string()),
        tags: Some(tags),
    };
    let _ = db.update_note(id, &upd);
    for old in db.folders_for_note(id).unwrap_or_default() {
        let _ = db.remove_note_from_folder(id, &old.id);
    }
    if let Some(fid) = folder_id {
        let _ = db.add_note_to_folder(id, fid);
    }
    db.get_note(id)
        .ok()
        .flatten()
        .map(|n| n.created_at)
        .unwrap_or_default()
}

/// The single place a transcription reaches `note_audios`, text and word timings
/// together.
///
/// `audio_id` names the clip when the caller knows it - which the recording path
/// does, since the row is created the moment recording stops. Falling back to
/// "the last audio of this note" is the pre-existing inference, kept only for
/// callers that genuinely have no id to give.
pub fn persist_last_transcription(
    db: &Database,
    note_id: &str,
    transcript: &Transcript,
    audio_id: Option<&str>,
) -> bool {
    let target = match audio_id {
        Some(id) => db
            .list_audios(note_id)
            .ok()
            .and_then(|a| a.into_iter().find(|audio| audio.id == id)),
        None => db.list_audios(note_id).ok().and_then(|a| a.last().cloned()),
    };
    let Some(target) = target else {
        return false;
    };
    if target.transcription.is_some() {
        return false;
    }
    let _ = db.set_audio_transcript(&target.id, transcript);
    true
}
