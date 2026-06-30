use crate::application::constants::RAG_DISTANCE_THRESHOLD;
use crate::infrastructure::vectordb::{SearchResult, SourceType};
use std::collections::HashSet;

/// A candidate clears the absolute relevance floor. Web rows are EXEMPT (the floor only gates
/// Local notes); a Local row passes only when its cosine `relevance` reaches the floor.
pub fn passes_floor(r: &SearchResult, floor: f32) -> bool {
    r.source_type != SourceType::Local || r.relevance >= floor
}

/// Keep only candidates that clear the floor (web exempt). Applied to RAW candidates BEFORE the
/// LLM rerank, so the abstain decision is deterministic and the rerank never truncates a relevant
/// hit out of reach.
pub fn floor_filter(results: &[SearchResult], floor: f32) -> Vec<SearchResult> {
    results
        .iter()
        .filter(|r| passes_floor(r, floor))
        .cloned()
        .collect()
}

pub(super) fn apply_temporal_boost(results: &mut [SearchResult]) {
    let now = chrono::Utc::now();
    for r in results.iter_mut() {
        let days_ago = chrono::DateTime::parse_from_rfc3339(&r.created_at)
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(
                    &r.created_at,
                    "%Y-%m-%dT%H:%M:%S",
                )
                .map(|dt| dt.and_utc().fixed_offset())
            })
            .map(|dt| (now - dt.with_timezone(&chrono::Utc)).num_days())
            .unwrap_or(365) as f32;
        let boost = 1.0 / (1.0 + days_ago / 30.0);
        r.distance *= 1.0 - (boost * 0.3);
    }
    results.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Collapse the relevant chunks into one card per NOTE: dedup by note_id (a note can yield several
/// chunks), keeping the relative `distance` belt. Relevance is decided upstream (the LLM judge +
/// keyword union), so this no longer filters on relevance - it only shapes the cited set.
pub(super) fn dedup_sources(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen = HashSet::new();
    results
        .into_iter()
        .filter(|r| r.distance <= RAG_DISTANCE_THRESHOLD)
        .filter(|r| seen.insert(r.note_id.clone()))
        .collect()
}

pub(super) fn compute_source_count(
    results: &[SearchResult],
    user_max: usize,
) -> usize {
    if results.is_empty() {
        return 0;
    }
    let avg_distance: f32 =
        results.iter().map(|r| r.distance).sum::<f32>() / results.len() as f32;
    let adaptive = if avg_distance < 0.3 {
        5
    } else if avg_distance < 0.5 {
        8
    } else {
        12
    };
    adaptive.min(user_max).min(results.len())
}
