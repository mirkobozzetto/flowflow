use crate::db::{now_iso, Database};
use crate::models::{NewTextNote, NewVoiceNote, Note, NoteType, UpdateNote};
use uuid::Uuid;

fn row_to_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    let tags_json: String = row.get("tags")?;
    let tags: Vec<String> =
        serde_json::from_str(&tags_json).unwrap_or_default();
    let note_type_str: String = row.get("note_type")?;
    Ok(Note {
        id: row.get("id")?,
        note_type: NoteType::from_str(&note_type_str),
        title: row.get("title")?,
        content: row.get("content")?,
        audio_file_path: row.get("audio_file_path")?,
        duration_secs: row.get("duration_secs")?,
        tags,
        created_at: row.get("created_at")?,
        modified_at: row.get("modified_at")?,
    })
}

impl Database {
    pub fn create_voice_note(
        &self,
        note: &NewVoiceNote,
    ) -> Result<Note, String> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let tags_json = serde_json::to_string(&note.tags)
            .unwrap_or_else(|_| "[]".to_string());
        self.conn()
            .execute(
                "INSERT INTO notes
                 (id, note_type, title, content, audio_file_path, duration_secs, tags, created_at, modified_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                rusqlite::params![
                    id, "voice", note.title, note.content, note.audio_file_path,
                    note.duration_secs, tags_json, now, now,
                ],
            )
            .map_err(|e| format!("Insert note: {e}"))?;
        self.get_note(&id)?
            .ok_or_else(|| "Note not found after insert".into())
    }

    pub fn create_text_note(&self, note: &NewTextNote) -> Result<Note, String> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let tags_json = serde_json::to_string(&note.tags)
            .unwrap_or_else(|_| "[]".to_string());
        self.conn()
            .execute(
                "INSERT INTO notes
                 (id, note_type, title, content, tags, created_at, modified_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                rusqlite::params![
                    id,
                    "text",
                    note.title,
                    note.content,
                    tags_json,
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
}
