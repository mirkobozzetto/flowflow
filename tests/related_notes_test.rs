// Pure tests for the related-notes v2 engine: per-note best-hit collapse,
// gap cutoff (ceiling/floor/largest-gap), keyword query building, RRF fusion.
// No LanceDB, no network.

use flowflow::application::related::{
    best_per_note, gap_cutoff, keyword_query, rrf_fuse,
};
use flowflow::infrastructure::vectordb::{SearchResult, SourceType};

fn hit(note_id: &str, title: &str, text: &str, distance: f32) -> SearchResult {
    SearchResult {
        chunk_text: text.into(),
        note_id: note_id.into(),
        title: title.into(),
        distance,
        relevance: (1.0 - distance).clamp(0.0, 1.0),
        created_at: "2026-06-01T10:00:00Z".into(),
        source_type: SourceType::Local,
        url: None,
    }
}

#[test]
fn best_per_note_excludes_self_and_keeps_best() {
    let hits = vec![
        hit("self", "Moi", "x", 0.0),
        hit("a", "A", "chunk 2", 0.4),
        hit("a", "A", "chunk 1", 0.2),
        hit("b", "B", "z", 0.3),
    ];
    let out = best_per_note("self", hits);
    let pairs: Vec<(&str, f32)> = out
        .iter()
        .map(|r| (r.note_id.as_str(), r.distance))
        .collect();
    assert_eq!(pairs, vec![("a", 0.2), ("b", 0.3)]);
}

#[test]
fn gap_cutoff_ceiling_drops_everything_far() {
    // Isolated note: nothing under the ceiling -> empty section.
    assert_eq!(gap_cutoff(&[0.8, 0.9, 0.95], 0.5, 0.75, 0.07), 0);
}

#[test]
fn gap_cutoff_cuts_at_largest_gap() {
    // Tight head cluster, then a wide jump: the tail goes.
    let d = [0.30, 0.35, 0.38, 0.65, 0.70];
    assert_eq!(gap_cutoff(&d, 0.5, 0.75, 0.07), 3);
}

#[test]
fn gap_cutoff_keeps_continuous_cluster() {
    // No jump wider than min_gap: one cluster, keep all under the ceiling.
    let d = [0.50, 0.55, 0.60, 0.65];
    assert_eq!(gap_cutoff(&d, 0.5, 0.75, 0.07), 4);
}

#[test]
fn gap_cutoff_floor_candidates_always_survive() {
    // The widest gap sits inside the floor zone: never cut there.
    let d = [0.10, 0.45, 0.48];
    assert_eq!(gap_cutoff(&d, 0.5, 0.75, 0.07), 3);
}

#[test]
fn gap_cutoff_single_close_candidate_kept() {
    assert_eq!(gap_cutoff(&[0.6], 0.5, 0.75, 0.07), 1);
    assert_eq!(gap_cutoff(&[], 0.5, 0.75, 0.07), 0);
}

#[test]
fn keyword_query_joins_title_and_tags() {
    assert_eq!(
        keyword_query("Rendez-vous Jean", "[\"budget\",\"jean\"]"),
        "Rendez-vous Jean budget jean"
    );
    assert_eq!(keyword_query("  ", "[]"), "");
    assert_eq!(keyword_query("", "not json"), "");
}

#[test]
fn rrf_fuse_note_in_both_legs_ranks_first() {
    let vector = vec![hit("both", "B", "x", 0.3), hit("vec", "V", "y", 0.4)];
    let keyword = vec![hit("kw", "K", "z", 0.1), hit("both", "B", "x", 0.2)];
    let fused = rrf_fuse(vector, keyword, 60.0);
    assert_eq!(fused[0].0.note_id, "both");
    let ids: Vec<&str> =
        fused.iter().map(|(r, _)| r.note_id.as_str()).collect();
    assert!(ids.contains(&"vec"));
    assert!(ids.contains(&"kw"));
}

#[test]
fn rrf_fuse_keyword_only_candidate_survives() {
    // The "Jean" case: no semantic overlap (absent from the vector leg),
    // linked anyway through the keyword leg.
    let vector = vec![hit("sem", "S", "x", 0.3)];
    let keyword = vec![hit("jean", "Jean", "y", 0.1)];
    let fused = rrf_fuse(vector, keyword, 60.0);
    assert_eq!(fused.len(), 2);
    assert!(fused.iter().any(|(r, _)| r.note_id == "jean"));
}

#[test]
fn rrf_fuse_vector_row_wins_for_display() {
    let vector = vec![hit("n", "Titre vecteur", "x", 0.3)];
    let keyword = vec![hit("n", "Titre keyword", "x", 0.5)];
    let fused = rrf_fuse(vector, keyword, 60.0);
    assert_eq!(fused.len(), 1);
    assert_eq!(fused[0].0.title, "Titre vecteur");
}
