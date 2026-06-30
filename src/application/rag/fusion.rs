use crate::application::constants::RAG_DISTANCE_THRESHOLD;
use crate::application::web_search::WebResult;
use crate::infrastructure::vectordb::{SearchResult, SourceType};
use std::collections::HashSet;

fn web_to_search_result(w: WebResult) -> SearchResult {
    SearchResult {
        chunk_text: w.snippet,
        note_id: String::new(),
        title: w.title,
        distance: 0.0,
        // Web rows are EXEMPT from the relevance floor (it only gates Local). 1.0 = always passes.
        relevance: 1.0,
        created_at: w.published_date.unwrap_or_default(),
        source_type: SourceType::Web,
        url: Some(w.url),
    }
}

pub fn rrf_merge(
    local: Vec<SearchResult>,
    web: Vec<WebResult>,
    k: f32,
    local_weight: f32,
    web_weight: f32,
) -> Vec<SearchResult> {
    let mut scored: Vec<(SearchResult, f32)> =
        Vec::with_capacity(local.len() + web.len());
    for (rank, r) in local.into_iter().enumerate() {
        scored.push((r, local_weight / (k + rank as f32 + 1.0)));
    }
    for (rank, w) in web.into_iter().enumerate() {
        scored.push((
            web_to_search_result(w),
            web_weight / (k + rank as f32 + 1.0),
        ));
    }
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.into_iter().map(|(r, _)| r).collect()
}

pub(super) fn dedup_merged(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen_notes = HashSet::new();
    let mut seen_web = HashSet::new();
    results
        .into_iter()
        .filter(|r| match r.source_type {
            SourceType::Local => {
                r.distance <= RAG_DISTANCE_THRESHOLD
                    && seen_notes.insert(r.note_id.clone())
            }
            SourceType::Web => {
                let key: String = r.chunk_text.chars().take(200).collect();
                seen_web.insert(key)
            }
        })
        .collect()
}
