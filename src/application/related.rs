use crate::application::ai::char_prefix;
use crate::application::constants::{
    RELATED_FETCH_K, RELATED_K, RELATED_MAX_DISTANCE,
};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::vectordb::{SearchResult, VectorStore};
use std::collections::HashSet;

/// One card of the "related notes" section on NoteDetail.
#[derive(Clone, Debug, PartialEq)]
pub struct RelatedNote {
    pub note_id: String,
    pub label: String,
    pub created_at: String,
}

/// Pick the related notes out of raw vector hits: drop the note itself (its own
/// chunks are always the top hits) and anything clearly unrelated, keep one row
/// per note, cap at `k`. Untitled notes fall back to a text excerpt.
pub fn select_related(
    self_id: &str,
    hits: Vec<SearchResult>,
    k: usize,
    max_distance: f32,
) -> Vec<RelatedNote> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for h in hits {
        if h.note_id == self_id || h.distance > max_distance {
            continue;
        }
        if !seen.insert(h.note_id.clone()) {
            continue;
        }
        let label = if h.title.trim().is_empty() {
            char_prefix(h.chunk_text.trim(), 60)
        } else {
            h.title
        };
        if label.is_empty() {
            continue;
        }
        out.push(RelatedNote {
            note_id: h.note_id,
            label,
            created_at: h.created_at,
        });
        if out.len() == k {
            break;
        }
    }
    out
}

/// Semantically closest notes to `note_id`, fully local: the query vector is the
/// note's stored first chunk (SQLite), the search runs on LanceDB. A note that was
/// never embedded (too short, pre-embedding) simply yields an empty section.
pub async fn related_notes(note_id: &str) -> Result<Vec<RelatedNote>, String> {
    let db = Database::open()?;
    let chunks = db.chunks_for_owner(note_id, "note")?;
    let Some(first) = chunks.into_iter().next() else {
        return Ok(vec![]);
    };
    let store = VectorStore::open().await?;
    let hits = store.search(first.vector, RELATED_FETCH_K, None).await?;
    Ok(select_related(
        note_id,
        hits,
        RELATED_K,
        RELATED_MAX_DISTANCE,
    ))
}
