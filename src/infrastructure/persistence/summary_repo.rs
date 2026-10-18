// A note's meeting summary. Its own table rather than a `notes` column: the
// notes update trigger would re-sync the whole note on every summary write.
// Synced as its own kind, so every device of the cluster keeps it. No foreign
// key on notes, as for note_shares (V25): the sync applier upserts notes with
// INSERT OR REPLACE, and a cascade would wipe the summary on every peer echo.
// The note delete path removes it. The key is named `id` (the note id)
// because the sync catalog locates every row by it.

use crate::domain::summary::{NoteSummary, Summary};
use crate::infrastructure::persistence::{now_iso, sync_meta, Database, DbTx};

pub(super) const V34_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS note_summaries (
    id TEXT PRIMARY KEY,
    body_json TEXT NOT NULL,
    lang TEXT NOT NULL,
    model TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    modified_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
";

impl Database {
    pub fn summary_for_note(&self, note_id: &str) -> Option<NoteSummary> {
        let (body, lang, model, source_hash): (String, String, String, String) =
            self.conn()
                .query_row(
                    "SELECT body_json, lang, model, source_hash
                     FROM note_summaries WHERE id = ?1",
                    [note_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .ok()?;
        Some(NoteSummary {
            note_id: note_id.to_string(),
            body: serde_json::from_str(&body).ok()?,
            lang,
            model,
            source_hash,
        })
    }

    /// Store a note's summary, replacing the one it had.
    pub fn save_summary(
        &self,
        note_id: &str,
        body: &Summary,
        lang: &str,
        model: &str,
        source_hash: &str,
    ) -> Result<(), String> {
        let json = serde_json::to_string(body)
            .map_err(|e| format!("Serialize summary: {e}"))?;
        self.conn()
            .execute(
                "INSERT INTO note_summaries
                    (id, body_json, lang, model, source_hash, modified_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    body_json = excluded.body_json, lang = excluded.lang,
                    model = excluded.model,
                    source_hash = excluded.source_hash,
                    modified_at = excluded.modified_at",
                rusqlite::params![
                    note_id,
                    json,
                    lang,
                    model,
                    source_hash,
                    now_iso()
                ],
            )
            .map_err(|e| format!("Save summary: {e}"))?;
        Ok(())
    }

    pub fn delete_summary(&self, note_id: &str) -> Result<(), String> {
        DbTx(&self.conn()).delete_summary(note_id)
    }
}

impl DbTx<'_> {
    /// Remove a note's summary, everywhere: the tombstone carries the delete
    /// to the other devices. Nothing to do when the note has none.
    pub fn delete_summary(&self, note_id: &str) -> Result<(), String> {
        self.savepoint("delete_summary", |tx| {
            let removed = tx
                .execute("DELETE FROM note_summaries WHERE id = ?1", [note_id])
                .map_err(|e| format!("Delete summary: {e}"))?;
            if removed > 0 {
                sync_meta::tombstone_entity(tx, "note_summary", note_id)?;
            }
            Ok(())
        })
    }
}
