use crate::db::{now_iso, Database};
use crate::models::{NewTextNote, Note, NoteAudio, NoteType, UpdateNote};
use std::str::FromStr;
use uuid::Uuid;

fn row_to_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    let tags_json: String = row.get("tags")?;
    let tags: Vec<String> =
        serde_json::from_str(&tags_json).unwrap_or_default();
    let note_type_str: String = row.get("note_type")?;
    Ok(Note {
        id: row.get("id")?,
        note_type: NoteType::from_str(&note_type_str).unwrap(),
        title: row.get("title")?,
        content: row.get("content")?,
        audio_file_path: row.get("audio_file_path")?,
        duration_secs: row.get("duration_secs")?,
        tags,
        created_at: row.get("created_at")?,
        modified_at: row.get("modified_at")?,
    })
}

fn row_to_note_audio(row: &rusqlite::Row) -> rusqlite::Result<NoteAudio> {
    Ok(NoteAudio {
        id: row.get("id")?,
        note_id: row.get("note_id")?,
        file_path: row.get("file_path")?,
        duration_secs: row.get("duration_secs")?,
        created_at: row.get("created_at")?,
    })
}

impl Database {
    pub fn create_text_note(&self, note: &NewTextNote) -> Result<Note, String> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let tags_json = serde_json::to_string(&note.tags)
            .unwrap_or_else(|_| "[]".to_string());
        self.conn()
            .execute(
                "INSERT INTO notes
                 (id, note_type, title, content, tags, audio_file_path, duration_secs, created_at, modified_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                rusqlite::params![
                    id,
                    "text",
                    note.title,
                    note.content,
                    tags_json,
                    note.audio_file_path,
                    note.duration_secs,
                    now,
                    now
                ],
            )
            .map_err(|e| format!("Insert note: {e}"))?;
        self.get_note(&id)?
            .ok_or_else(|| "Note not found after insert".into())
    }

    pub fn get_note(&self, id: &str) -> Result<Option<Note>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM notes WHERE id = ?1")
            .map_err(|e| format!("Prepare: {e}"))?;
        let mut rows = stmt
            .query_map([id], row_to_note)
            .map_err(|e| format!("Query: {e}"))?;
        match rows.next() {
            Some(Ok(note)) => Ok(Some(note)),
            Some(Err(e)) => Err(format!("Row: {e}")),
            None => Ok(None),
        }
    }

    pub fn list_notes(&self) -> Result<Vec<Note>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM notes ORDER BY created_at DESC")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], row_to_note)
            .map_err(|e| format!("Query: {e}"))?;
        let mut notes = Vec::new();
        for row in rows {
            notes.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(notes)
    }

    pub fn list_notes_in_folder(
        &self,
        folder_id: &str,
    ) -> Result<Vec<Note>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT n.* FROM notes n
                 JOIN notes_folders nf ON nf.note_id = n.id
                 WHERE nf.folder_id = ?1
                 ORDER BY n.created_at DESC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([folder_id], row_to_note)
            .map_err(|e| format!("Query: {e}"))?;
        let mut notes = Vec::new();
        for row in rows {
            notes.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(notes)
    }

    pub fn update_note(
        &self,
        id: &str,
        update: &UpdateNote,
    ) -> Result<(), String> {
        let conn = self.conn();
        let now = now_iso();
        if let Some(ref title) = update.title {
            conn.execute(
                "UPDATE notes SET title = ?1, modified_at = ?2 WHERE id = ?3",
                rusqlite::params![title, now, id],
            )
            .map_err(|e| format!("Update title: {e}"))?;
        }
        if let Some(ref content) = update.content {
            conn.execute(
                "UPDATE notes SET content = ?1, modified_at = ?2 WHERE id = ?3",
                rusqlite::params![content, now, id],
            )
            .map_err(|e| format!("Update content: {e}"))?;
        }
        if let Some(ref tags) = update.tags {
            let tags_json = serde_json::to_string(tags)
                .unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "UPDATE notes SET tags = ?1, modified_at = ?2 WHERE id = ?3",
                rusqlite::params![tags_json, now, id],
            )
            .map_err(|e| format!("Update tags: {e}"))?;
        }
        Ok(())
    }

    pub fn delete_note(&self, id: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM notes WHERE id = ?1", [id])
            .map_err(|e| format!("Delete note: {e}"))?;
        Ok(())
    }

    pub fn add_audio(
        &self,
        note_id: &str,
        file_path: &str,
        duration_secs: f64,
    ) -> Result<NoteAudio, String> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        self.conn()
            .execute(
                "INSERT INTO note_audios (id, note_id, file_path, duration_secs, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, note_id, file_path, duration_secs, now],
            )
            .map_err(|e| format!("Insert audio: {e}"))?;
        Ok(NoteAudio {
            id,
            note_id: note_id.to_string(),
            file_path: file_path.to_string(),
            duration_secs: Some(duration_secs),
            created_at: now,
        })
    }

    pub fn list_audios(&self, note_id: &str) -> Result<Vec<NoteAudio>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM note_audios WHERE note_id = ?1 ORDER BY created_at ASC")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([note_id], row_to_note_audio)
            .map_err(|e| format!("Query: {e}"))?;
        let mut audios = Vec::new();
        for row in rows {
            audios.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(audios)
    }

    pub fn delete_audio(&self, id: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM note_audios WHERE id = ?1", [id])
            .map_err(|e| format!("Delete audio: {e}"))?;
        Ok(())
    }

    pub fn all_audio_paths(&self) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT file_path FROM note_audios")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Query: {e}"))?;
        let mut paths = Vec::new();
        for row in rows {
            paths.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(paths)
    }

    pub fn cleanup_orphan_audio(&self, audio_dir: &str) {
        let known: std::collections::HashSet<String> = self
            .all_audio_paths()
            .unwrap_or_default()
            .into_iter()
            .collect();
        let dir = std::path::Path::new(audio_dir);
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("wav") {
                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_string();
                    if !known.contains(&filename) {
                        let _ = std::fs::remove_file(&path);
                        eprintln!("[cleanup] removed orphan: {filename}");
                    }
                }
            }
        }
    }

    pub fn delete_all_audios(&self, audio_dir: &str) -> Result<usize, String> {
        let paths = self.all_audio_paths().unwrap_or_default();
        let dir = std::path::Path::new(audio_dir);
        for p in &paths {
            let _ = std::fs::remove_file(dir.join(p));
        }
        let count = paths.len();
        self.conn()
            .execute("DELETE FROM note_audios", [])
            .map_err(|e| format!("Delete all audios: {e}"))?;
        Ok(count)
    }

    pub fn list_all_tags(&self) -> Vec<String> {
        let notes = self.list_notes().unwrap_or_default();
        let mut tags: Vec<String> =
            notes.into_iter().flat_map(|n| n.tags).collect();
        tags.sort();
        tags.dedup();
        tags
    }
}
