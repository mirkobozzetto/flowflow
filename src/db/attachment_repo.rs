use crate::db::{now_iso, sync_meta, Database};
use crate::models::{Attachment, NewAttachment};
use uuid::Uuid;

fn row_to_attachment(row: &rusqlite::Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get("id")?,
        note_id: row.get("note_id")?,
        filename: row.get("filename")?,
        content_text: row.get("content_text")?,
        imported_at: row.get("imported_at")?,
    })
}

impl Database {
    pub fn create_attachment(
        &self,
        attachment: &NewAttachment,
    ) -> Result<Attachment, String> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        self.conn()
            .execute(
                "INSERT INTO attachments
                 (id, note_id, filename, content_text, imported_at)
                 VALUES (?1,?2,?3,?4,?5)",
                rusqlite::params![
                    id,
                    attachment.note_id,
                    attachment.filename,
                    attachment.content_text,
                    now
                ],
            )
            .map_err(|e| format!("Insert attachment: {e}"))?;
        self.get_attachment(&id)?
            .ok_or_else(|| "Attachment not found after insert".into())
    }

    pub fn get_attachment(
        &self,
        id: &str,
    ) -> Result<Option<Attachment>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM attachments WHERE id = ?1")
            .map_err(|e| format!("Prepare: {e}"))?;
        let mut rows = stmt
            .query_map([id], row_to_attachment)
            .map_err(|e| format!("Query: {e}"))?;
        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(format!("Row: {e}")),
            None => Ok(None),
        }
    }

    pub fn list_attachments_for_note(
        &self,
        note_id: &str,
    ) -> Result<Vec<Attachment>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM attachments
                 WHERE note_id = ?1
                 ORDER BY imported_at ASC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([note_id], row_to_attachment)
            .map_err(|e| format!("Query: {e}"))?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(items)
    }

    pub fn delete_attachment(&self, id: &str) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Delete attachment tx: {e}"))?;
        let n = tx
            .execute("DELETE FROM attachments WHERE id = ?1", [id])
            .map_err(|e| format!("Delete attachment: {e}"))?;
        if n > 0 {
            sync_meta::tombstone_entity(&tx, "attachment", id)?;
        }
        tx.commit()
            .map_err(|e| format!("Delete attachment commit: {e}"))?;
        Ok(())
    }

    pub fn delete_attachments_for_note(
        &self,
        note_id: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Delete attachments tx: {e}"))?;
        for id in sync_meta::collect_ids(
            &tx,
            "SELECT id FROM attachments WHERE note_id = ?1",
            note_id,
        ) {
            sync_meta::tombstone_entity(&tx, "attachment", &id)?;
        }
        tx.execute("DELETE FROM attachments WHERE note_id = ?1", [note_id])
            .map_err(|e| format!("Delete attachments: {e}"))?;
        tx.commit()
            .map_err(|e| format!("Delete attachments commit: {e}"))?;
        Ok(())
    }
}
