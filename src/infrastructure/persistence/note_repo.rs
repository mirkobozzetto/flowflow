use crate::domain::{NewTextNote, Note, NoteAudio, NoteType, UpdateNote};
use crate::infrastructure::persistence::{
    chunk_repo, now_iso, sync_meta, Database,
};
use std::str::FromStr;
use uuid::Uuid;

pub(crate) fn row_to_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    let tags_json: String = row.get("tags")?;
    let tags: Vec<String> =
        serde_json::from_str(&tags_json).unwrap_or_default();
    let note_type_str: String = row.get("note_type")?;
    Ok(Note {
        id: row.get("id")?,
        note_type: NoteType::from_str(&note_type_str).unwrap(),
        title: row.get("title")?,
        content: row.get("content")?,
        tags,
        sources_json: row.get("sources_json")?,
        thread_id: row.get("thread_id")?,
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
        transcription: row.get("transcription")?,
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

    pub fn create_text_note_in_thread(
        &self,
        note: &NewTextNote,
        thread_id: &str,
    ) -> Result<Note, String> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let tags_json = serde_json::to_string(&note.tags)
            .unwrap_or_else(|_| "[]".to_string());
        self.conn()
            .execute(
                "INSERT INTO notes
                 (id, note_type, title, content, tags, thread_id, created_at, modified_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                rusqlite::params![
                    id,
                    "text",
                    note.title,
                    note.content,
                    tags_json,
                    thread_id,
                    now,
                    now
                ],
            )
            .map_err(|e| format!("Insert note in thread: {e}"))?;
        self.get_note(&id)?
            .ok_or_else(|| "Note not found after insert".into())
    }

    pub fn set_note_sources(
        &self,
        note_id: &str,
        sources_json: Option<&str>,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE notes SET sources_json = ?1 WHERE id = ?2",
                rusqlite::params![sources_json, note_id],
            )
            .map_err(|e| format!("Set note sources: {e}"))?;
        Ok(())
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

    pub fn list_root_notes(&self) -> Result<Vec<Note>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM notes n
                 WHERE n.thread_id IS NULL
                    OR (SELECT COUNT(*) FROM notes m WHERE m.thread_id = n.thread_id) < 2
                 ORDER BY n.created_at DESC",
            )
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

    pub fn list_root_notes_in_folder(
        &self,
        folder_id: &str,
    ) -> Result<Vec<Note>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT n.* FROM notes n
                 JOIN notes_folders nf ON nf.note_id = n.id
                 WHERE nf.folder_id = ?1
                   AND (n.thread_id IS NULL
                        OR (SELECT COUNT(*) FROM notes m WHERE m.thread_id = n.thread_id) < 2)
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
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Delete note tx: {e}"))?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM notes WHERE id = ?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Delete note check: {e}"))?;
        if !exists {
            return Ok(());
        }
        // Tombstone the note + every CASCADE child BEFORE the physical delete,
        // so a deletion propagates and never resurrects on the peer. CASCADE
        // still does the physical cleanup.
        for child in sync_meta::collect_ids(
            &tx,
            "SELECT id FROM attachments WHERE note_id = ?1",
            id,
        ) {
            sync_meta::tombstone_entity(&tx, "attachment", &child)?;
            chunk_repo::delete_chunks_for_owner(&tx, &child, "attachment")
                .map_err(|e| format!("Delete attachment chunks: {e}"))?;
        }
        for child in sync_meta::collect_ids(
            &tx,
            "SELECT id FROM note_audios WHERE note_id = ?1",
            id,
        ) {
            sync_meta::tombstone_entity(&tx, "note_audio", &child)?;
        }
        for child in sync_meta::collect_ids(
            &tx,
            "SELECT id FROM note_reminders WHERE note_id = ?1",
            id,
        ) {
            sync_meta::tombstone_entity(&tx, "note_reminder", &child)?;
        }
        for link in sync_meta::collect_links(
            &tx,
            "SELECT folder_id, note_id FROM notes_folders WHERE note_id = ?1",
            id,
        ) {
            sync_meta::tombstone_entity(&tx, "notes_folders", &link)?;
        }
        sync_meta::tombstone_entity(&tx, "note", id)?;
        chunk_repo::delete_chunks_for_owner(&tx, id, "note")
            .map_err(|e| format!("Delete note chunks: {e}"))?;
        tx.execute("DELETE FROM notes WHERE id = ?1", [id])
            .map_err(|e| format!("Delete note: {e}"))?;
        tx.commit()
            .map_err(|e| format!("Delete note commit: {e}"))?;
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
            transcription: None,
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

    pub fn set_audio_transcription(
        &self,
        audio_id: &str,
        text: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE note_audios SET transcription = ?1 WHERE id = ?2",
                rusqlite::params![text, audio_id],
            )
            .map_err(|e| format!("Update transcription: {e}"))?;
        Ok(())
    }

    pub fn has_any_audio(&self, note_id: &str) -> bool {
        self.conn()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM note_audios WHERE note_id = ?1)",
                [note_id],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false)
    }

    pub fn delete_audio(&self, id: &str) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Delete audio tx: {e}"))?;
        let n = tx
            .execute("DELETE FROM note_audios WHERE id = ?1", [id])
            .map_err(|e| format!("Delete audio: {e}"))?;
        if n > 0 {
            sync_meta::tombstone_entity(&tx, "note_audio", id)?;
        }
        tx.commit()
            .map_err(|e| format!("Delete audio commit: {e}"))?;
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
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Delete all audios tx: {e}"))?;
        let ids: Vec<String> = {
            let mut stmt = tx
                .prepare("SELECT id FROM note_audios")
                .map_err(|e| format!("List audio ids: {e}"))?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| format!("Query audio ids: {e}"))?;
            rows.flatten().collect()
        };
        for id in ids {
            sync_meta::tombstone_entity(&tx, "note_audio", &id)?;
        }
        tx.execute("DELETE FROM note_audios", [])
            .map_err(|e| format!("Delete all audios: {e}"))?;
        tx.commit()
            .map_err(|e| format!("Delete all audios commit: {e}"))?;
        Ok(count)
    }

    // Hard local wipe for "delete my data" (RFC 0009 Q1.6). RAW deletes (not tombstones) so nothing
    // propagates to former cluster peers, and the sync lineage is cleared so the device restarts as a
    // fresh solo install. Settings (device identity, API keys, backend URL) are preserved so the app
    // still boots; the caller removes the returned audio files and purges the vector store.
    pub fn wipe_local_content(&self) -> Result<Vec<String>, String> {
        let audio_paths = self.all_audio_paths().unwrap_or_default();
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("wipe tx: {e}"))?;
        for table in [
            "note_audios",
            "attachments",
            "note_reminders",
            "conversation_messages",
            "conversations",
            "notes_folders",
            "chunks",
            "pending_transcriptions",
            "notes",
            "threads",
            "folders",
            "sync_row_meta",
            "sync_seq",
            "sync_conflicts",
            "sync_peers",
        ] {
            tx.execute(&format!("DELETE FROM {table}"), [])
                .map_err(|e| format!("wipe {table}: {e}"))?;
        }
        tx.commit().map_err(|e| format!("wipe commit: {e}"))?;
        Ok(audio_paths)
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
