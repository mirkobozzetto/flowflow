use flowflow::services::tools::{
    CreateNote, CreateNoteArgs, CreateNoteResult, SearchNotesArgs,
    SearchNotesHit, SummarizeFolderArgs, SummarizeFolderResult, ToolFailure,
};
use rig::tool::Tool;

#[test]
fn test_tool_failure_display() {
    let err = ToolFailure("something went wrong".to_string());
    assert_eq!(format!("{err}"), "something went wrong");
}

#[test]
fn test_tool_failure_debug_format() {
    let err = ToolFailure("oops".to_string());
    let dbg = format!("{err:?}");
    assert!(dbg.contains("ToolFailure"));
    assert!(dbg.contains("oops"));
}

#[test]
fn test_tool_failure_is_std_error() {
    fn assert_is_error<E: std::error::Error>(_: &E) {}
    let err = ToolFailure("x".into());
    assert_is_error(&err);
}

#[test]
fn test_tool_failure_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ToolFailure>();
}

#[test]
fn test_search_notes_args_deser_valid() {
    let json = r#"{"query": "réunion budget", "top_k": 3}"#;
    let args: SearchNotesArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.query, "réunion budget");
    assert_eq!(args.top_k, Some(3));
}

#[test]
fn test_search_notes_args_deser_without_top_k() {
    let json = r#"{"query": "dentiste"}"#;
    let args: SearchNotesArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.query, "dentiste");
    assert_eq!(args.top_k, None);
}

#[test]
fn test_search_notes_args_deser_missing_query() {
    let json = r#"{"top_k": 5}"#;
    let result: Result<SearchNotesArgs, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_search_notes_hit_serialize() {
    let hit = SearchNotesHit {
        note_id: "n1".to_string(),
        title: "Mon titre".to_string(),
        excerpt: "Contenu du chunk".to_string(),
        distance: 0.42,
    };
    let json = serde_json::to_value(&hit).unwrap();
    assert_eq!(json["note_id"], "n1");
    assert_eq!(json["title"], "Mon titre");
    assert_eq!(json["excerpt"], "Contenu du chunk");
    assert!((json["distance"].as_f64().unwrap() - 0.42).abs() < 1e-5);
}

#[tokio::test]
async fn test_create_note_definition() {
    let tool = CreateNote::new();
    let def = tool.definition(String::new()).await;
    assert_eq!(def.name, "create_note");
    assert!(!def.description.is_empty());
    let required = def.parameters["required"].as_array().unwrap();
    assert!(required.iter().any(|v| v == "content"));
    assert!(def.parameters["properties"]["content"].is_object());
    assert!(def.parameters["properties"]["title"].is_object());
    assert!(def.parameters["properties"]["tags"].is_object());
}

#[test]
fn test_create_note_args_deser_minimal() {
    let json = r#"{"content": "Just a quick note"}"#;
    let args: CreateNoteArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.content, "Just a quick note");
    assert!(args.title.is_none());
    assert!(args.tags.is_none());
}

#[test]
fn test_create_note_args_deser_full() {
    let json = r#"{
        "title": "Idée projet",
        "content": "Lancer un SaaS",
        "tags": ["projet", "saas", "idée"]
    }"#;
    let args: CreateNoteArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.title.as_deref(), Some("Idée projet"));
    assert_eq!(args.content, "Lancer un SaaS");
    assert_eq!(
        args.tags.as_deref(),
        Some(
            &["projet".to_string(), "saas".to_string(), "idée".to_string()][..]
        )
    );
}

#[test]
fn test_create_note_args_deser_missing_content() {
    let json = r#"{"title": "no content"}"#;
    let result: Result<CreateNoteArgs, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_create_note_result_serialize() {
    let result = CreateNoteResult {
        note_id: "uuid-123".to_string(),
        title: Some("Mon titre".to_string()),
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["note_id"], "uuid-123");
    assert_eq!(json["title"], "Mon titre");
}

#[test]
fn test_create_note_result_serialize_null_title() {
    let result = CreateNoteResult {
        note_id: "uuid-456".to_string(),
        title: None,
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["note_id"], "uuid-456");
    assert!(json["title"].is_null());
}

#[test]
fn test_summarize_folder_args_deser_valid() {
    let json = r#"{"folder_name": "Projets", "max_notes": 10}"#;
    let args: SummarizeFolderArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.folder_name, "Projets");
    assert_eq!(args.max_notes, Some(10));
}

#[test]
fn test_summarize_folder_args_deser_without_max() {
    let json = r#"{"folder_name": "Idées"}"#;
    let args: SummarizeFolderArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.folder_name, "Idées");
    assert_eq!(args.max_notes, None);
}

#[test]
fn test_summarize_folder_args_deser_missing_name() {
    let json = r#"{"max_notes": 5}"#;
    let result: Result<SummarizeFolderArgs, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_summarize_folder_result_serialize() {
    let result = SummarizeFolderResult {
        folder_name: "Projets".to_string(),
        note_count: 7,
        summary: "Résumé du dossier".to_string(),
    };
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["folder_name"], "Projets");
    assert_eq!(json["note_count"], 7);
    assert_eq!(json["summary"], "Résumé du dossier");
}

#[test]
fn test_create_note_default_impl() {
    let _tool = CreateNote;
    let _tool2 = CreateNote::default();
    let _tool3 = CreateNote::new();
}

#[test]
fn test_create_note_clone() {
    let tool = CreateNote::new();
    let _cloned = tool.clone();
}
