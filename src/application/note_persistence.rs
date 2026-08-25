use crate::domain::{
    merge_transcript_into_body, NewTextNote, Note, UpdateNote,
};
use crate::infrastructure::persistence::{Database, DbTx};

/// Returns the created note and, when a clip was attached, its `note_audios`
/// id - the caller needs it to enqueue a transcription against this exact clip
/// rather than guessing "the note's last audio".
pub fn create_note(
    db: &Database,
    title: &str,
    content: &str,
    tags: Vec<String>,
    folder_id: Option<&str>,
    audio: Option<(String, f64)>,
) -> Option<(Note, Option<String>)> {
    db.with_tx(|tx| create_note_in(tx, title, content, tags, folder_id, audio))
        .ok()
}

/// `create_note` inside a caller's unit of work (a space page).
pub fn create_note_in(
    tx: &DbTx,
    title: &str,
    content: &str,
    tags: Vec<String>,
    folder_id: Option<&str>,
    audio: Option<(String, f64)>,
) -> Result<(Note, Option<String>), String> {
    let new = NewTextNote {
        title: if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        },
        content: content.to_string(),
        tags,
    };
    let created = tx.create_text_note(&new)?;
    if let Some(fid) = folder_id {
        tx.add_note_to_folder(&created.id, fid)?;
    }
    let audio_id = match audio {
        Some((path, dur)) => Some(tx.add_audio(&created.id, &path, dur)?.id),
        None => None,
    };
    Ok((created, audio_id))
}

/// Removes the note and everything only it owns: its audio files on disk, its
/// vector chunks, and its share/provenance rows (FK-free by design, so they
/// need explicit cleanup). Rows that cascade (attachments, reminders) are
/// left to SQLite. Sync scheduling stays with the caller, which owns the
/// session; proposing a share revoke is the UI's job before calling this.
pub fn delete_note(db: &Database, id: &str) {
    let audio_paths = db
        .with_tx(|tx| delete_note_rows(tx, id))
        .unwrap_or_default();
    finish_note_delete(db, id, &audio_paths);
}

/// The SQL half of `delete_note`, for a caller's unit of work. Returns the
/// audio files the caller removes once its transaction has committed: a file
/// deleted before a rollback is a file lost.
pub fn delete_note_rows(tx: &DbTx, id: &str) -> Result<Vec<String>, String> {
    let paths = tx
        .list_audios(id)?
        .iter()
        .map(|a| crate::infrastructure::audio::resolve_audio_path(&a.file_path))
        .collect();
    tx.delete_note(id)?;
    tx.delete_provenance(id)?;
    if tx.get_share(id).is_some() {
        tx.delete_share(id)?;
    }
    Ok(paths)
}

/// The out-of-SQLite half of `delete_note`: audio files and vector chunks.
pub fn finish_note_delete(db: &Database, id: &str, audio_paths: &[String]) {
    for path in audio_paths {
        let _ = std::fs::remove_file(path);
    }
    crate::application::embed::delete_note_embeddings(db, id);
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

/// What a committed transcript leaves the caller: the merged body to show, and
/// the note fields `embed_note` needs, so no caller re-reads the row.
pub struct CommittedBody {
    pub merged: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created_at: String,
}

/// The single path a transcript takes into `notes.content`.
///
/// `current_body` is the editor's live value when a note is open, the stored
/// body otherwise. `None` means nothing was written - blank transcript, or the
/// note is gone.
///
/// Re-embedding is the caller's job. `embed_note` detaches an OS thread that
/// opens the app-wide `Database` on its own, so keeping it out of here is what
/// makes this function hermetic and testable against a temp DB; the returned
/// `CommittedBody` carries everything `embed_note` asks for.
pub fn commit_transcription(
    db: &Database,
    note_id: &str,
    current_body: &str,
    text: &str,
) -> Option<CommittedBody> {
    let merged = merge_transcript_into_body(current_body, text)?;
    let note = db.get_note(note_id).ok().flatten()?;
    db.update_note(
        note_id,
        &UpdateNote {
            title: None,
            content: Some(merged.clone()),
            tags: None,
        },
    )
    .ok()?;
    Some(CommittedBody {
        merged,
        title: note.title.unwrap_or_default(),
        tags: note.tags,
        created_at: note.created_at,
    })
}

/// `commit_transcription` for callers with no open editor: the stored body is
/// the current one. This one owns its re-embedding, since it has no UI to hand
/// the `CommittedBody` back to.
pub fn append_transcription_to_note(db: &Database, note_id: &str, text: &str) {
    let Ok(Some(note)) = db.get_note(note_id) else {
        return;
    };
    let Some(c) = commit_transcription(db, note_id, &note.content, text) else {
        return;
    };
    crate::application::embed::embed_note(
        note_id.to_string(),
        c.title,
        c.merged,
        c.tags,
        c.created_at,
    );
}
