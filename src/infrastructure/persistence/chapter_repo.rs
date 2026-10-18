// Chapters of a recording, stored on its word-timings row because they index
// those words. The row's INSERT OR REPLACE on every new transcription resets
// them to NULL, so a retranscribed clip gets fresh chapters. Not synced, like
// the words.

use crate::domain::reading::Chapter;
use crate::infrastructure::persistence::Database;

pub(super) const V33_SCHEMA: &str = "
ALTER TABLE note_audio_words ADD COLUMN chapters_json TEXT;
";

impl Database {
    /// The chapters of a clip, `None` until they were generated.
    pub fn chapters_for_audio(&self, audio_id: &str) -> Option<Vec<Chapter>> {
        let json: Option<String> = self
            .conn()
            .query_row(
                "SELECT chapters_json FROM note_audio_words WHERE audio_id = ?1",
                [audio_id],
                |row| row.get(0),
            )
            .ok()?;
        serde_json::from_str(&json?).ok()
    }

    /// Store a clip's chapters, computed from its `word_count` words. Returns
    /// whether they were kept: a transcript replaced while they were being
    /// generated, or chapters already there, leave the row untouched.
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
                "UPDATE note_audio_words SET chapters_json = ?2
                 WHERE audio_id = ?1 AND chapters_json IS NULL
                   AND json_array_length(words_json) = ?3",
                rusqlite::params![audio_id, json, word_count as i64],
            )
            .map_err(|e| format!("Set chapters: {e}"))?;
        Ok(changed == 1)
    }
}
