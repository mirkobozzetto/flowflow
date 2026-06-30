// Pure-function tests for the relevance floor + grounded abstain (RFC 0015). No embed, no
// OpenAI key, no live VectorStore - every seam under test is a pure fn over plain data, so this
// runs in CI offline. End-to-end `rag::query` stays behind `#[ignore]` in rag_integration_test.

use flowflow::application::constants::RAG_RELEVANCE_FLOOR;
use flowflow::application::rag::{
    abstain_decision, floor_filter, passes_floor, AbstainDecision,
};
use flowflow::infrastructure::vectordb::{
    cosine, note_id_in_filter, table_state, SearchResult, SourceType,
    TableState,
};

fn local(relevance: f32) -> SearchResult {
    SearchResult {
        chunk_text: "c".into(),
        note_id: "n".into(),
        title: "t".into(),
        distance: 0.0,
        relevance,
        created_at: String::new(),
        source_type: SourceType::Local,
        url: None,
    }
}

fn web() -> SearchResult {
    SearchResult {
        chunk_text: "w".into(),
        note_id: String::new(),
        title: "web".into(),
        distance: 0.0,
        relevance: 1.0,
        created_at: String::new(),
        source_type: SourceType::Web,
        url: Some("https://x".into()),
    }
}

// --- cosine (T02) ---

#[test]
fn cosine_identical_is_one() {
    let v = vec![1.0, 2.0, 3.0];
    assert!((cosine(&v, &v) - 1.0).abs() < 1e-6);
}

#[test]
fn cosine_orthogonal_is_zero() {
    assert_eq!(cosine(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
}

#[test]
fn cosine_zero_norm_is_zero() {
    assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    assert_eq!(cosine(&[1.0, 1.0], &[0.0, 0.0]), 0.0);
}

#[test]
fn cosine_opposite_clamps_to_zero() {
    // raw cosine = -1, clamped to [0,1]
    assert_eq!(cosine(&[1.0, 0.0], &[-1.0, 0.0]), 0.0);
}

#[test]
fn cosine_length_mismatch_is_zero() {
    assert_eq!(cosine(&[1.0, 2.0], &[1.0]), 0.0);
}

#[test]
fn cosine_stays_in_unit_range() {
    let a = vec![0.3, 0.4, 0.5, 0.1];
    let b = vec![0.1, 0.9, 0.2, 0.4];
    let c = cosine(&a, &b);
    assert!((0.0..=1.0).contains(&c), "cosine {c} out of [0,1]");
}

// --- floor_filter / passes_floor (T03) ---

#[test]
fn floor_drops_local_below_keeps_local_above() {
    let floor = 0.25;
    assert!(!passes_floor(&local(0.10), floor));
    assert!(passes_floor(&local(0.25), floor)); // boundary inclusive
    assert!(passes_floor(&local(0.40), floor));
}

#[test]
fn floor_exempts_web_rows() {
    let floor = 0.99;
    // a web row with relevance below the floor is still kept (exempt)
    let mut w = web();
    w.relevance = 0.0;
    assert!(passes_floor(&w, floor));
}

#[test]
fn floor_filter_keeps_supra_and_web_drops_sub() {
    let floor = 0.25;
    let input = vec![local(0.10), local(0.30), web(), local(0.05)];
    let kept = floor_filter(&input, floor);
    assert_eq!(kept.len(), 2, "the 0.30 local + the web row survive");
    assert!(kept.iter().any(|r| r.source_type == SourceType::Web));
    assert!(kept
        .iter()
        .any(|r| r.source_type == SourceType::Local && r.relevance >= floor));
}

#[test]
fn floor_filter_all_below_is_empty() {
    let kept = floor_filter(&[local(0.10), local(0.05)], 0.25);
    assert!(kept.is_empty());
}

#[test]
fn default_floor_is_the_documented_constant() {
    assert_eq!(RAG_RELEVANCE_FLOOR, 0.25);
}

// --- abstain_decision matrix (T04) ---

#[test]
fn abstain_only_when_empty_weboff_no_intent() {
    // the one Abstain case
    assert_eq!(
        abstain_decision(true, false, false, false),
        AbstainDecision::Abstain
    );
}

#[test]
fn proceed_when_floor_kept_something() {
    assert_eq!(
        abstain_decision(false, false, false, false),
        AbstainDecision::Proceed
    );
}

#[test]
fn proceed_when_mention_even_if_empty() {
    assert_eq!(
        abstain_decision(true, false, true, false),
        AbstainDecision::Proceed
    );
}

#[test]
fn proceed_when_actionable_even_if_empty() {
    assert_eq!(
        abstain_decision(true, false, false, true),
        AbstainDecision::Proceed
    );
}

#[test]
fn proceed_when_web_on_even_if_empty() {
    // web-on never abstains via this helper (web-empty is decided separately)
    assert_eq!(
        abstain_decision(true, true, false, false),
        AbstainDecision::Proceed
    );
}

// --- note_id_in_filter escaping (T07) ---

#[test]
fn id_filter_quotes_and_joins() {
    let f = note_id_in_filter(&["a".into(), "b".into()]);
    assert_eq!(f, "note_id IN ('a','b')");
}

#[test]
fn id_filter_escapes_single_quotes() {
    let f = note_id_in_filter(&["o'brien".into()]);
    assert_eq!(f, "note_id IN ('o''brien')");
}

// --- table_state branches (T08) ---

#[test]
fn table_present_when_listed() {
    let names = vec!["chunks".to_string(), "other".to_string()];
    assert_eq!(table_state(&names, "chunks"), TableState::Present);
}

#[test]
fn table_missing_on_fresh_install() {
    // a zero-note user has no table - legitimate empty, NOT an error
    assert_eq!(table_state(&[], "chunks"), TableState::Missing);
    assert_eq!(
        table_state(&["other".to_string()], "chunks"),
        TableState::Missing
    );
}
