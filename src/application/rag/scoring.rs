use crate::application::constants::RAG_DISTANCE_THRESHOLD;
use crate::infrastructure::vectordb::SearchResult;
use std::collections::HashSet;

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

pub(super) fn filter_and_dedup(
    results: Vec<SearchResult>,
) -> Vec<SearchResult> {
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
