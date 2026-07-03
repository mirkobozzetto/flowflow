// Pure tests for the related-notes selection: exclude self, dedup per note,
// distance gate, cap, untitled label fallback. No LanceDB, no network.

use flowflow::application::related::select_related;
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
fn excludes_the_note_itself() {
    let hits = vec![
        hit("self", "Moi", "x", 0.0),
        hit("other", "Autre", "y", 0.3),
    ];
    let out = select_related("self", hits, 3, 0.75);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].note_id, "other");
}

#[test]
fn dedups_multiple_chunks_of_the_same_note() {
    let hits = vec![
        hit("a", "A", "chunk 1", 0.2),
        hit("a", "A", "chunk 2", 0.25),
        hit("b", "B", "z", 0.3),
    ];
    let out = select_related("self", hits, 3, 0.75);
    let ids: Vec<&str> = out.iter().map(|r| r.note_id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b"]);
}

#[test]
fn distance_gate_cuts_unrelated_hits() {
    let hits = vec![
        hit("near", "Proche", "x", 0.4),
        hit("far", "Loin", "y", 0.9),
    ];
    let out = select_related("self", hits, 3, 0.75);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].note_id, "near");
}

#[test]
fn caps_at_k_keeping_closest_first() {
    let hits: Vec<SearchResult> = (0..6)
        .map(|i| {
            hit(
                &format!("n{i}"),
                &format!("T{i}"),
                "x",
                0.1 + i as f32 * 0.05,
            )
        })
        .collect();
    let out = select_related("self", hits, 3, 0.75);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].note_id, "n0");
}

#[test]
fn untitled_note_gets_excerpt_label() {
    let hits = vec![hit(
        "u",
        "  ",
        "une idée de pitch pour la startup demain",
        0.3,
    )];
    let out = select_related("self", hits, 3, 0.75);
    assert_eq!(out.len(), 1);
    assert!(out[0].label.starts_with("une idée de pitch"));
}

#[test]
fn empty_label_rows_are_dropped() {
    let hits = vec![hit("e", "", "   ", 0.2)];
    let out = select_related("self", hits, 3, 0.75);
    assert!(out.is_empty());
}
