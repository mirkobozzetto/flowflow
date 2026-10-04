use crate::domain::Word;
use crate::infrastructure::persistence::Database;

pub struct PendingTranscription {
    pub note_id: String,
    pub transcription_id: Option<String>,
    pub soniox_file_id: Option<String>,
    pub provider: String,
    pub file_path: Option<String>,
    /// The recorded clip this job belongs to. `None` for an import, which has
    /// no `note_audios` row to anchor word timings to.
    pub audio_id: Option<String>,
}

/// Where a local job stopped. `file_path` tells which recording it belongs to.
pub struct LocalProgress {
    pub file_path: Option<String>,
    pub done_ms: u32,
    pub words: Vec<Word>,
}

impl Database {
    pub fn add_pending_transcription(
        &self,
        note_id: &str,
        transcription_id: &str,
        soniox_file_id: Option<&str>,
        audio_id: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO pending_transcriptions
                (note_id, transcription_id, soniox_file_id, provider,
                 file_path, audio_id)
             VALUES (?1, ?2, ?3, 'soniox', NULL, ?4)
             ON CONFLICT(note_id) DO UPDATE SET
                transcription_id = excluded.transcription_id,
                soniox_file_id = excluded.soniox_file_id,
                provider = excluded.provider,
                file_path = excluded.file_path,
                audio_id = excluded.audio_id",
            rusqlite::params![
                note_id,
                transcription_id,
                soniox_file_id,
                audio_id
            ],
        )
        .map_err(|e| format!("Add pending transcription: {e}"))?;
        Ok(())
    }

    /// Written before the upload, so a job killed mid-upload is sent again on
    /// the next launch. `add_pending_transcription` replaces it once Soniox
    /// has the file.
    pub fn add_pending_soniox_upload(
        &self,
        note_id: &str,
        file_path: &str,
        audio_id: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO pending_transcriptions
                (note_id, transcription_id, soniox_file_id, provider,
                 file_path, audio_id)
             VALUES (?1, NULL, NULL, 'soniox', ?2, ?3)
             ON CONFLICT(note_id) DO UPDATE SET
                transcription_id = excluded.transcription_id,
                soniox_file_id = excluded.soniox_file_id,
                provider = excluded.provider,
                file_path = excluded.file_path,
                audio_id = excluded.audio_id",
            rusqlite::params![note_id, file_path, audio_id],
        )
        .map_err(|e| format!("Add pending soniox upload: {e}"))?;
        Ok(())
    }

    /// Leaves `done_ms` and `words_json` alone on conflict: a resumed job
    /// re-registers itself and must keep its progress.
    pub fn add_pending_local_transcription(
        &self,
        note_id: &str,
        file_path: &str,
        audio_id: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO pending_transcriptions
                (note_id, transcription_id, soniox_file_id, provider,
                 file_path, audio_id)
             VALUES (?1, NULL, NULL, 'whisper_local', ?2, ?3)
             ON CONFLICT(note_id) DO UPDATE SET
                transcription_id = excluded.transcription_id,
                soniox_file_id = excluded.soniox_file_id,
                provider = excluded.provider,
                file_path = excluded.file_path,
                audio_id = excluded.audio_id",
            rusqlite::params![note_id, file_path, audio_id],
        )
        .map_err(|e| format!("Add pending local transcription: {e}"))?;
        Ok(())
    }

    pub fn list_pending_transcriptions(&self) -> Vec<PendingTranscription> {
        let conn = self.conn();
        let mut stmt = match conn.prepare(
            "SELECT note_id, transcription_id, soniox_file_id, provider,
                    file_path, audio_id
             FROM pending_transcriptions",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |row| {
            Ok(PendingTranscription {
                note_id: row.get(0)?,
                transcription_id: row.get(1)?,
                soniox_file_id: row.get(2)?,
                provider: row.get(3)?,
                file_path: row.get(4)?,
                audio_id: row.get(5)?,
            })
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    pub fn save_local_progress(
        &self,
        note_id: &str,
        done_ms: u32,
        words: &[Word],
    ) -> Result<(), String> {
        let json = serde_json::to_string(words)
            .map_err(|e| format!("Encode local progress: {e}"))?;
        self.conn()
            .execute(
                "UPDATE pending_transcriptions SET done_ms = ?2, words_json = ?3
                 WHERE note_id = ?1",
                rusqlite::params![note_id, done_ms, json],
            )
            .map_err(|e| format!("Save local progress: {e}"))?;
        Ok(())
    }

    pub fn local_progress(&self, note_id: &str) -> Option<LocalProgress> {
        let (file_path, done_ms, json) = self
            .conn()
            .query_row(
                "SELECT file_path, done_ms, words_json FROM pending_transcriptions
                 WHERE note_id = ?1 AND provider = 'whisper_local'",
                [note_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .ok()?;
        let words = match json {
            Some(j) => serde_json::from_str(&j).ok()?,
            None => Vec::new(),
        };
        Some(LocalProgress {
            file_path,
            done_ms,
            words,
        })
    }

    pub fn delete_pending_transcription(
        &self,
        note_id: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM pending_transcriptions WHERE note_id = ?1",
            [note_id],
        )
        .map_err(|e| format!("Delete pending transcription: {e}"))?;
        Ok(())
    }
}
