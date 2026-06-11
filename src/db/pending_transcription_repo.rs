use crate::db::Database;

pub struct PendingTranscription {
    pub note_id: String,
    pub transcription_id: Option<String>,
    pub soniox_file_id: Option<String>,
    pub provider: String,
    pub file_path: Option<String>,
}

impl Database {
    pub fn add_pending_transcription(
        &self,
        note_id: &str,
        transcription_id: &str,
        soniox_file_id: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO pending_transcriptions
                (note_id, transcription_id, soniox_file_id, provider,
                 file_path)
             VALUES (?1, ?2, ?3, 'soniox', NULL)
             ON CONFLICT(note_id) DO UPDATE SET
                transcription_id = excluded.transcription_id,
                soniox_file_id = excluded.soniox_file_id,
                provider = excluded.provider,
                file_path = excluded.file_path",
            rusqlite::params![note_id, transcription_id, soniox_file_id],
        )
        .map_err(|e| format!("Add pending transcription: {e}"))?;
        Ok(())
    }

    pub fn add_pending_local_transcription(
        &self,
        note_id: &str,
        file_path: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO pending_transcriptions
                (note_id, transcription_id, soniox_file_id, provider,
                 file_path)
             VALUES (?1, NULL, NULL, 'whisper_local', ?2)
             ON CONFLICT(note_id) DO UPDATE SET
                transcription_id = excluded.transcription_id,
                soniox_file_id = excluded.soniox_file_id,
                provider = excluded.provider,
                file_path = excluded.file_path",
            rusqlite::params![note_id, file_path],
        )
        .map_err(|e| format!("Add pending local transcription: {e}"))?;
        Ok(())
    }

    pub fn list_pending_transcriptions(&self) -> Vec<PendingTranscription> {
        let conn = self.conn();
        let mut stmt = match conn.prepare(
            "SELECT note_id, transcription_id, soniox_file_id, provider,
                    file_path
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
            })
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
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
