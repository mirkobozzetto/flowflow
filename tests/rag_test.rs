use flowflow::application::rag::{build_context, RagResponse, RagSource};
use flowflow::infrastructure::vectordb::{SearchResult, SourceType};

fn make_source(
    note_id: &str,
    title: &str,
    text: &str,
    distance: f32,
) -> RagSource {
    RagSource {
        note_id: note_id.to_string(),
        title: title.to_string(),
        chunk_text: text.to_string(),
        distance,
        created_at: String::new(),
        source_type: SourceType::Local,
        url: None,
    }
}

fn make_result(
    note_id: &str,
    title: &str,
    text: &str,
    distance: f32,
) -> SearchResult {
    SearchResult {
        note_id: note_id.to_string(),
        title: title.to_string(),
        chunk_text: text.to_string(),
        distance,
        relevance: (1.0 - distance).clamp(0.0, 1.0),
        created_at: String::new(),
        source_type: SourceType::Local,
        url: None,
    }
}

#[test]
fn test_rag_source_fields() {
    let src = make_source("note-1", "Titre", "Contenu du chunk", 0.42);
    assert_eq!(src.note_id, "note-1");
    assert_eq!(src.title, "Titre");
    assert_eq!(src.chunk_text, "Contenu du chunk");
    assert!((src.distance - 0.42).abs() < f32::EPSILON);
}

#[test]
fn test_rag_source_clone() {
    let src = make_source("note-1", "Titre", "Contenu", 0.1);
    let copy = src.clone();
    assert_eq!(src.note_id, copy.note_id);
    assert_eq!(src.title, copy.title);
    assert_eq!(src.chunk_text, copy.chunk_text);
    assert_eq!(src.distance, copy.distance);
}

#[test]
fn test_rag_response_empty_sources() {
    let resp = RagResponse {
        answer: "Aucune note trouvée".to_string(),
        sources: vec![],
        trace: None,
    };
    assert_eq!(resp.answer, "Aucune note trouvée");
    assert!(resp.sources.is_empty());
}

#[test]
fn test_rag_response_with_sources() {
    let resp = RagResponse {
        answer: "Réponse synthétisée".to_string(),
        sources: vec![
            make_source("n1", "T1", "C1", 0.1),
            make_source("n2", "T2", "C2", 0.2),
        ],
        trace: None,
    };
    assert_eq!(resp.sources.len(), 2);
    assert_eq!(resp.sources[0].note_id, "n1");
    assert_eq!(resp.sources[1].note_id, "n2");
}

#[test]
fn test_rag_response_clone() {
    let resp = RagResponse {
        answer: "Réponse".to_string(),
        sources: vec![make_source("n1", "T1", "C1", 0.5)],
        trace: None,
    };
    let copy = resp.clone();
    assert_eq!(resp.answer, copy.answer);
    assert_eq!(resp.sources.len(), copy.sources.len());
    assert_eq!(resp.sources[0].note_id, copy.sources[0].note_id);
}

#[test]
fn test_build_context_empty() {
    let ctx = build_context(&[]);
    assert_eq!(ctx, "--- User notes ---\n\n");
}

#[test]
fn test_build_context_single_source() {
    let results = vec![make_result("n1", "Mon titre", "Contenu du chunk", 0.1)];
    let ctx = build_context(&results);
    let expected = "--- User notes ---\n\n[Source 1] Note: \"Mon titre\"\nContenu du chunk\n\n";
    assert_eq!(ctx, expected);
}

#[test]
fn test_build_context_multiple_sources_numbered() {
    let results = vec![
        make_result("n1", "Note A", "Texte A", 0.1),
        make_result("n2", "Note B", "Texte B", 0.2),
        make_result("n3", "Note C", "Texte C", 0.3),
    ];
    let ctx = build_context(&results);
    assert!(ctx.contains("[Source 1] Note: \"Note A\"\nTexte A"));
    assert!(ctx.contains("[Source 2] Note: \"Note B\"\nTexte B"));
    assert!(ctx.contains("[Source 3] Note: \"Note C\"\nTexte C"));
    assert!(ctx.starts_with("--- User notes ---"));
}

#[test]
fn test_build_context_preserves_order() {
    let results = vec![
        make_result("n1", "First", "F", 0.1),
        make_result("n2", "Second", "S", 0.2),
    ];
    let ctx = build_context(&results);
    let first_pos = ctx.find("[Source 1]").unwrap();
    let second_pos = ctx.find("[Source 2]").unwrap();
    assert!(first_pos < second_pos);
}

#[test]
fn test_build_context_special_chars_in_title() {
    let results = vec![make_result(
        "n1",
        "Titre avec \"guillemets\"",
        "Contenu",
        0.1,
    )];
    let ctx = build_context(&results);
    assert!(ctx.contains("[Source 1] Note: \"Titre avec \"guillemets\"\""));
}
