use crate::db::{now_iso, sync_meta, Database};
use crate::models::conversation::{Conversation, ConversationMessage};

fn chat_scope_key(conversation_id: &str) -> String {
    format!("chat_scope:{conversation_id}")
}

impl Database {
    pub fn chat_scope(&self, conversation_id: &str) -> Option<String> {
        self.get_setting(&chat_scope_key(conversation_id))
            .filter(|v| !v.is_empty())
    }

    pub fn set_chat_scope(
        &self,
        conversation_id: &str,
        folder_id: Option<&str>,
    ) -> Result<(), String> {
        self.set_setting(
            &chat_scope_key(conversation_id),
            folder_id.unwrap_or(""),
        )
    }

    pub fn create_conversation(
        &self,
        title: &str,
    ) -> Result<Conversation, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_iso();
        let conn = self.conn();
        conn.execute(
            "INSERT INTO conversations (id, title, created_at, modified_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, title, now, now],
        )
        .map_err(|e| format!("Create conversation: {e}"))?;
        Ok(Conversation {
            id,
            title: title.to_string(),
            created_at: now.clone(),
            modified_at: now,
        })
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id, title, created_at, modified_at FROM conversations ORDER BY modified_at DESC")
            .map_err(|e| format!("List conversations: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    modified_at: row.get(3)?,
                })
            })
            .map_err(|e| format!("Query conversations: {e}"))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Delete conversation tx: {e}"))?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = ?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Delete conversation check: {e}"))?;
        if !exists {
            return Ok(());
        }
        for msg in sync_meta::collect_ids(
            &tx,
            "SELECT id FROM conversation_messages WHERE conversation_id = ?1",
            id,
        ) {
            sync_meta::tombstone_entity(&tx, "conversation_message", &msg)?;
        }
        sync_meta::tombstone_entity(&tx, "conversation", id)?;
        tx.execute("DELETE FROM conversations WHERE id = ?1", [id])
            .map_err(|e| format!("Delete conversation: {e}"))?;
        tx.execute("DELETE FROM settings WHERE key = ?1", [chat_scope_key(id)])
            .map_err(|e| format!("Delete conversation scope: {e}"))?;
        tx.commit()
            .map_err(|e| format!("Delete conversation commit: {e}"))?;
        Ok(())
    }

    pub fn update_conversation_title(
        &self,
        id: &str,
        title: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        let now = now_iso();
        conn.execute(
            "UPDATE conversations SET title = ?1, modified_at = ?2 WHERE id = ?3",
            rusqlite::params![title, now, id],
        )
        .map_err(|e| format!("Update conversation title: {e}"))?;
        Ok(())
    }

    pub fn touch_conversation(&self, id: &str) -> Result<(), String> {
        let conn = self.conn();
        let now = now_iso();
        conn.execute(
            "UPDATE conversations SET modified_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )
        .map_err(|e| format!("Touch conversation: {e}"))?;
        Ok(())
    }

    pub fn add_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        sources_json: Option<&str>,
    ) -> Result<ConversationMessage, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_iso();
        let conn = self.conn();
        conn.execute(
            "INSERT INTO conversation_messages (id, conversation_id, role, content, sources_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, conversation_id, role, content, sources_json, now],
        )
        .map_err(|e| format!("Add message: {e}"))?;
        Ok(ConversationMessage {
            id,
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            sources_json: sources_json.map(String::from),
            created_at: now,
        })
    }

    pub fn list_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ConversationMessage>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id, conversation_id, role, content, sources_json, created_at FROM conversation_messages WHERE conversation_id = ?1 ORDER BY created_at ASC")
            .map_err(|e| format!("List messages: {e}"))?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(ConversationMessage {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    sources_json: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| format!("Query messages: {e}"))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
