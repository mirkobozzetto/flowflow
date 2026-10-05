use crate::domain::conversation::Conversation;
use crate::infrastructure::persistence::{now_iso, Database};

// Hermes conversations are device-local pointers to Hermes sessions: the
// history lives in Hermes, so the rows stay out of sync and backups.
impl Database {
    pub fn create_hermes_conversation(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<Conversation, String> {
        let now = now_iso();
        self.conn()
            .execute(
                "INSERT INTO hermes_conversations (session_id, title, created_at, modified_at) VALUES (?1, ?2, ?3, ?3)",
                rusqlite::params![session_id, title, now],
            )
            .map_err(|e| format!("Create Hermes conversation: {e}"))?;
        Ok(Conversation {
            id: session_id.to_string(),
            title: title.to_string(),
            created_at: now.clone(),
            modified_at: now,
        })
    }

    pub fn list_hermes_conversations(
        &self,
    ) -> Result<Vec<Conversation>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT session_id, title, created_at, modified_at FROM hermes_conversations ORDER BY modified_at DESC")
            .map_err(|e| format!("List Hermes conversations: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    modified_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("Query Hermes conversations: {e}"))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn rename_hermes_conversation(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE hermes_conversations SET title = ?1, modified_at = ?2 WHERE session_id = ?3",
                rusqlite::params![title, now_iso(), session_id],
            )
            .map_err(|e| format!("Rename Hermes conversation: {e}"))?;
        Ok(())
    }

    pub fn touch_hermes_conversation(
        &self,
        session_id: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE hermes_conversations SET modified_at = ?1 WHERE session_id = ?2",
                rusqlite::params![now_iso(), session_id],
            )
            .map_err(|e| format!("Touch Hermes conversation: {e}"))?;
        Ok(())
    }

    /// Forgets the conversation here; the session stays in Hermes.
    pub fn delete_hermes_conversation(
        &self,
        session_id: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "DELETE FROM hermes_conversations WHERE session_id = ?1",
                [session_id],
            )
            .map_err(|e| format!("Delete Hermes conversation: {e}"))?;
        Ok(())
    }

    /// The run still answering and its question, or None to clear it.
    pub fn set_hermes_pending_run(
        &self,
        session_id: &str,
        run: Option<(&str, &str)>,
    ) -> Result<(), String> {
        let (run_id, input) = run.unzip();
        self.conn()
            .execute(
                "UPDATE hermes_conversations SET pending_run_id = ?1, pending_input = ?2 WHERE session_id = ?3",
                rusqlite::params![run_id, input, session_id],
            )
            .map_err(|e| format!("Set Hermes pending run: {e}"))?;
        Ok(())
    }

    /// (run id, question) of the turn still running.
    pub fn hermes_pending_run(
        &self,
        session_id: &str,
    ) -> Option<(String, String)> {
        self.conn()
            .query_row(
                "SELECT pending_run_id, pending_input FROM hermes_conversations WHERE session_id = ?1 AND pending_run_id IS NOT NULL",
                [session_id],
                |row| Ok((row.get(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default())),
            )
            .ok()
    }
}
