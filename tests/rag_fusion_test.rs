use flowflow::application::rag::rrf_merge;
use flowflow::application::web_search::WebResult;
use flowflow::infrastructure::vectordb::{SearchResult, SourceType};

fn local(title: &str, distance: f32) -> SearchResult {
    SearchResult {
        chunk_text: format!("text-{title}"),
        note_id: format!("id-{title}"),
        title: title.into(),
        distance,
        relevance: (1.0 - distance).clamp(0.0, 1.0),
        created_at: "2026-01-01T00:00:00".into(),
        source_type: SourceType::Local,
        url: None,
    }
}

fn web(title: &str) -> WebResult {
    WebResult {
        title: title.into(),
        url: format!("https://{title}"),
        snippet: format!("snip-{title}"),
        published_date: None,
    }
}

fn titles(rs: &[SearchResult]) -> Vec<String> {
    rs.iter().map(|r| r.title.clone()).collect()
}

#[test]
fn empty_web_is_identity() {
    let l = vec![local("a", 0.1), local("b", 0.2), local("c", 0.3)];
    let merged = rrf_merge(l, vec![], 60.0, 1.2, 1.0);
    assert_eq!(titles(&merged), ["a", "b", "c"]);
    assert_eq!(merged[0].distance, 0.1);
    assert!(merged.iter().all(|r| r.source_type == SourceType::Local));
}

#[test]
fn empty_local_keeps_web_order() {
    let merged = rrf_merge(vec![], vec![web("w1"), web("w2")], 60.0, 1.2, 1.0);
    assert_eq!(titles(&merged), ["w1", "w2"]);
    assert!(merged
        .iter()
        .all(|r| r.source_type == SourceType::Web && r.url.is_some()));
}

#[test]
fn local_outranks_web_at_same_rank() {
    let merged =
        rrf_merge(vec![local("L0", 0.1)], vec![web("W0")], 60.0, 1.2, 1.0);
    assert_eq!(titles(&merged), ["L0", "W0"]);
}

#[test]
fn web_top_interleaves_below_first_local() {
    let locals = vec![local("L0", 0.1), local("L1", 0.2), local("L2", 0.3)];
    let merged = rrf_merge(locals, vec![web("W0")], 1.0, 1.2, 1.0);
    assert_eq!(titles(&merged), ["L0", "W0", "L1", "L2"]);
}
