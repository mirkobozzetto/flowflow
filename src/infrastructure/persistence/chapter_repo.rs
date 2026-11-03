// Chapters of a recording, keyed like its words by the clip id. Words and
// chapters are two synced kinds, so two tables: the applier's INSERT OR
// REPLACE writes only its own kind's columns and would reset the other's.
// A new transcription removes the chapters (note_repo), so a retranscribed
// clip gets fresh ones.

use crate::domain::reading::Chapter;
use crate::infrastructure::persistence::{sync_meta, Database};

const WORDS: &str = "note_audio_words";
const CHAPTERS: &str = "note_audio_chapters";

/// Remove one synced row of a clip, with the tombstone that carries the delete
/// to the other devices. Table and sync kind share the name.
fn delete_clip_row(
    tx: &rusqlite::Connection,
    table: &str,
    audio_id: &str,
) -> Result<(), String> {
    let removed = tx
        .execute(&format!("DELETE FROM {table} WHERE id = ?1"), [audio_id])
        .map_err(|e| format!("Delete {table}: {e}"))?;
    if removed > 0 {
        sync_meta::tombstone_entity(tx, table, audio_id)?;
    }
    Ok(())
}

pub(crate) fn delete_chapters(
    tx: &rusqlite::Connection,
    audio_id: &str,
) -> Result<(), String> {
    delete_clip_row(tx, CHAPTERS, audio_id)
}

/// The word and chapter rows deliberately have no foreign key (see migration
/// V20), so every path that removes a `note_audios` row goes through here. A
/// missed path is not an orphan nuisance: on the wipe path it is a privacy defect.
pub(crate) fn delete_words_for_audio(
    tx: &rusqlite::Connection,
    audio_id: &str,
) -> Result<(), String> {
    delete_clip_row(tx, WORDS, audio_id)?;
    delete_chapters(tx, audio_id)
}

pub(super) const V33_SCHEMA: &str = "
ALTER TABLE note_audio_words ADD COLUMN chapters_json TEXT;
";

// `id` because the sync catalog finds every row by `id = ?1`; no foreign key,
// for the same reason as V20. author_device: the device that transcribed the
// clip, the only one that makes its chapters.
pub(super) const V35_SCHEMA: &str = "
ALTER TABLE note_audio_words RENAME COLUMN audio_id TO id;
ALTER TABLE note_audio_words ADD COLUMN author_device TEXT;
CREATE TABLE IF NOT EXISTS note_audio_chapters (
    id TEXT PRIMARY KEY,
    chapters_json TEXT NOT NULL
);
INSERT INTO note_audio_chapters (id, chapters_json)
    SELECT id, chapters_json FROM note_audio_words
    WHERE chapters_json IS NOT NULL;
ALTER TABLE note_audio_words DROP COLUMN chapters_json;
";

impl Database {
    /// The device that transcribed a clip, `None` without words.
    pub fn words_author(&self, audio_id: &str) -> Option<String> {
        self.conn()
            .query_row(
                "SELECT author_device FROM note_audio_words WHERE id = ?1",
                [audio_id],
                |row| row.get(0),
            )
            .ok()
            .flatten()
    }

    /// The chapters of a clip, `None` until they were generated.
    pub fn chapters_for_audio(&self, audio_id: &str) -> Option<Vec<Chapter>> {
        let json: String = self
            .conn()
            .query_row(
                "SELECT chapters_json FROM note_audio_chapters WHERE id = ?1",
                [audio_id],
                |row| row.get(0),
            )
            .ok()?;
        serde_json::from_str(&json).ok()
    }

    /// Store a clip's chapters, computed from its `word_count` words. Returns
    /// whether they were kept: a transcript replaced while they were being
    /// generated, or chapters already there, leave the store untouched.
    pub fn set_audio_chapters(
        &self,
        audio_id: &str,
        word_count: usize,
        chapters: &[Chapter],
    ) -> Result<bool, String> {
        let json = serde_json::to_string(chapters)
            .map_err(|e| format!("Serialize chapters: {e}"))?;
        // ponytail: a retranscription is told apart by its word count; one
        // with exactly as many words would keep the old chapters.
        let changed = self
            .conn()
            .execute(
                "INSERT INTO note_audio_chapters (id, chapters_json)
                 SELECT ?1, ?2 WHERE EXISTS (
                     SELECT 1 FROM note_audio_words
                     WHERE id = ?1 AND json_array_length(words_json) = ?3)
                 ON CONFLICT(id) DO NOTHING",
                rusqlite::params![audio_id, json, word_count as i64],
            )
            .map_err(|e| format!("Set chapters: {e}"))?;
        Ok(changed == 1)
    }
}
