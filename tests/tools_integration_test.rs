use flowflow::db::Database;
use flowflow::models::{NewFolder, NewTextNote};
use flowflow::services::tools::{
    CreateNote, CreateNoteArgs, SearchNotesArgs, SummarizeFolderArgs,
};
use rig::tool::Tool;
use tokio::sync::Mutex;
use uuid::Uuid;

static DB_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_create_note_persists_in_db() {
    let _guard = DB_LOCK.lock().await;
    let tool = CreateNote::new();
    let marker = format!("test-marker-{}", Uuid::new_v4());
    let args = CreateNoteArgs {
        title: Some("Integration test note".to_string()),
        content: marker.clone(),
        tags: Some(vec!["test".to_string(), "integration".to_string()]),
    };

    let result = tool.call(args).await.expect("create_note should succeed");
    assert!(!result.note_id.is_empty());
    assert_eq!(result.title.as_deref(), Some("Integration test note"));

    let db = Database::open().expect("open db");
    let note = db
        .get_note(&result.note_id)
        .expect("query note")
        .expect("note should exist");
    assert_eq!(note.content, marker);
    assert_eq!(note.title.as_deref(), Some("Integration test note"));
    assert!(note.tags.contains(&"test".to_string()));
    assert!(note.tags.contains(&"integration".to_string()));

    db.delete_note(&result.note_id).expect("cleanup note");
}

#[tokio::test]
async fn test_create_note_returns_valid_uuid() {
    let _guard = DB_LOCK.lock().await;
    let tool = CreateNote::new();
    let args = CreateNoteArgs {
        title: None,
        content: "Short content for uuid test".to_string(),
        tags: None,
    };

    let result = tool.call(args).await.expect("create_note should succeed");
    let parsed = Uuid::parse_str(&result.note_id);
    assert!(
        parsed.is_ok(),
        "note_id must be a valid UUID, got: {}",
        result.note_id
    );

    let db = Database::open().expect("open db");
    db.delete_note(&result.note_id).expect("cleanup");
}

#[tokio::test]
async fn test_create_note_minimal_no_title_no_tags() {
    let _guard = DB_LOCK.lock().await;
    let tool = CreateNote::new();
    let args = CreateNoteArgs {
        title: None,
        content: "Just content".to_string(),
        tags: None,
    };

    let result = tool.call(args).await.expect("create_note should succeed");
    assert!(result.title.is_none());

    let db = Database::open().expect("open db");
    let note = db
        .get_note(&result.note_id)
        .expect("query")
        .expect("exists");
    assert_eq!(note.content, "Just content");
    assert!(note.title.is_none());
    assert!(note.tags.is_empty());

    db.delete_note(&result.note_id).expect("cleanup");
}

#[tokio::test]
async fn test_summarize_folder_missing_folder_returns_error() {
    use flowflow::services::llm::LlmClient;
    use flowflow::services::tools::SummarizeFolder;
    use std::sync::Arc;

    let llm = match LlmClient::from_env() {
        Ok(c) => Arc::new(c),
        Err(_) => return,
    };
    let _guard = DB_LOCK.lock().await;
    let tool = SummarizeFolder::new(llm);
    let args = SummarizeFolderArgs {
        folder_name: format!("__never_exists_{}__", Uuid::new_v4()),
        max_notes: None,
    };

    let result = tool.call(args).await;
    assert!(result.is_err(), "should fail when folder not found");
    let err = result.err().unwrap();
    assert!(err.to_string().contains("folder not found"));
}

#[tokio::test]
async fn test_summarize_folder_empty_folder() {
    use flowflow::services::llm::LlmClient;
    use flowflow::services::tools::SummarizeFolder;
    use std::sync::Arc;

    let llm = match LlmClient::from_env() {
        Ok(c) => Arc::new(c),
        Err(_) => return,
    };

    let _guard = DB_LOCK.lock().await;
    let db = Database::open().expect("open db");
    let folder_name = format!("empty-folder-{}", Uuid::new_v4());
    let folder = db
        .create_folder(&NewFolder {
            name: folder_name.clone(),
            description: None,
            parent_id: None,
        })
        .expect("create folder");

    let tool = SummarizeFolder::new(llm);
    let args = SummarizeFolderArgs {
        folder_name: folder_name.clone(),
        max_notes: None,
    };
    let result = tool.call(args).await.expect("summarize empty folder");

    assert_eq!(result.folder_name, folder_name);
    assert_eq!(result.note_count, 0);
    assert_eq!(result.summary, "This folder is empty.");

    db.delete_folder(&folder.id).expect("cleanup folder");
}

#[tokio::test]
#[ignore]
async fn test_search_notes_real() {
    use flowflow::services::llm::LlmClient;
    use flowflow::services::tools::SearchNotes;
    use std::sync::Arc;

    let llm = Arc::new(LlmClient::from_env().expect("OPENAI_API_KEY required"));
    let tool = SearchNotes::new(llm);
    let args = SearchNotesArgs {
        query: "réunion projet".to_string(),
        top_k: Some(3),
    };

    let hits = tool.call(args).await.expect("search should succeed");
    assert!(hits.len() <= 3);
    for hit in &hits {
        assert!(!hit.note_id.is_empty());
        assert!(hit.distance >= 0.0);
    }
}

#[tokio::test]
#[ignore]
async fn test_summarize_folder_real() {
    use flowflow::services::llm::LlmClient;
    use flowflow::services::tools::SummarizeFolder;
    use std::sync::Arc;

    let llm = Arc::new(LlmClient::from_env().expect("OPENAI_API_KEY required"));
    let _guard = DB_LOCK.lock().await;
    let db = Database::open().expect("open db");

    let folder_name = format!("summarize-test-{}", Uuid::new_v4());
    let folder = db
        .create_folder(&NewFolder {
            name: folder_name.clone(),
            description: None,
            parent_id: None,
        })
        .expect("create folder");

    let note = db
        .create_text_note(&NewTextNote {
            title: Some("Test note".to_string()),
            content: "Le projet avance bien cette semaine. \
                Réunion prévue mardi pour faire le point."
                .to_string(),
            tags: vec![],
        })
        .expect("create note");
    db.add_note_to_folder(&note.id, &folder.id).expect("link");

    let tool = SummarizeFolder::new(llm);
    let args = SummarizeFolderArgs {
        folder_name: folder_name.clone(),
        max_notes: None,
    };
    let result = tool.call(args).await.expect("summarize should succeed");

    assert_eq!(result.folder_name, folder_name);
    assert_eq!(result.note_count, 1);
    assert!(!result.summary.is_empty());

    db.delete_note(&note.id).expect("cleanup note");
    db.delete_folder(&folder.id).expect("cleanup folder");
}

#[tokio::test]
#[ignore]
async fn test_agent_with_tools_e2e() {
    use flowflow::services::llm::LlmClient;
    use flowflow::services::tools::prompt_agent_with_tools;
    use std::sync::Arc;

    let llm = Arc::new(LlmClient::from_env().expect("OPENAI_API_KEY required"));
    let answer = prompt_agent_with_tools(
        llm,
        "Tu es un assistant qui répond brièvement en français.",
        "Bonjour, peux-tu juste dire 'pong' ?",
        None,
    )
    .await
    .expect("agent prompt should succeed");

    assert!(
        !answer.trim().is_empty(),
        "agent response must not be empty"
    );
}
