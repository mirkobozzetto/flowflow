use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Where something was said: one of the note's recordings, and a time in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Moment {
    pub audio_id: String,
    pub ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryItem {
    pub text: String,
    #[serde(default)]
    pub at: Option<Moment>,
}

/// A meeting summary, in four parts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    #[serde(default)]
    pub key_points: Vec<SummaryItem>,
    #[serde(default)]
    pub decisions: Vec<SummaryItem>,
    #[serde(default)]
    pub actions: Vec<SummaryItem>,
    #[serde(default)]
    pub questions: Vec<SummaryItem>,
}

impl Summary {
    pub fn is_empty(&self) -> bool {
        self.key_points.is_empty()
            && self.decisions.is_empty()
            && self.actions.is_empty()
            && self.questions.is_empty()
    }
}

/// A note's summary as kept: `source_hash` fingerprints the body it was made
/// from, so a later edit shows.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteSummary {
    pub note_id: String,
    pub body: Summary,
    pub lang: String,
    pub model: String,
    pub source_hash: String,
}

/// The fingerprint of a note body. SHA-256, not the std hasher, whose output
/// may change between Rust releases: another device compares it with its own
/// copy of the body.
pub fn source_hash(content: &str) -> String {
    Sha256::digest(content.trim().as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
