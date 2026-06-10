use flowflow::services::constants::EMBEDDING_DIMS;
use flowflow::services::vectordb::{Chunk, VectorStore};
use std::sync::Once;
use tokio::sync::OnceCell;
use uuid::Uuid;

static TABLE_READY: OnceCell<()> = OnceCell::const_new();
static SEAM: Once = Once::new();

// On macOS the global store now resolves to ~/Library/Application Support
// (desktop fix #20). Point this test binary at a scratch dir so it never
// touches the real user vector index / database.
fn isolate_store() {
    SEAM.call_once(|| {
        let dir = std::env::temp_dir().join("flowflow-test-rag");
        std::env::set_var("FLOWFLOW_DATA_DIR", &dir);
        std::env::set_var("FLOWFLOW_VECTORDB_PATH", dir.join("vectordb"));
    });
}

const PRIMER_NOTE_ID: &str = "__test_primer__";

fn unit_vector(slot: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; EMBEDDING_DIMS];
    v[slot % EMBEDDING_DIMS] = 1.0;
    v
}

fn make_chunk(
    note_id: &str,
    chunk_index: i32,
    text: &str,
    title: &str,
    vector: Vec<f32>,
) -> Chunk {
    Chunk {
        id: format!("note:{note_id}:{chunk_index}"),
        note_id: note_id.to_string(),
        chunk_text: text.to_string(),
        chunk_index,
        vector,
        title: title.to_string(),
        tags: String::new(),
        created_at: "2026-05-11T00:00:00Z".to_string(),
    }
}

async fn ensure_table() -> VectorStore {
    isolate_store();
    let store = VectorStore::open().await.expect("open vectordb");
    TABLE_READY
        .get_or_init(|| async {
            let primer = vec![make_chunk(
                PRIMER_NOTE_ID,
                0,
                "primer",
                "primer",
                unit_vector(0),
            )];
            let _ = store.store_chunks(primer).await;
        })
        .await;
    store
}

async fn cleanup(store: &VectorStore, note_id: &str) {
    let _ = store.delete_note_chunks(note_id).await;
}

#[tokio::test]
async fn test_vectordb_store_and_search() {
    let store = ensure_table().await;
    let note_id = Uuid::new_v4().to_string();

    let chunks = vec![
        make_chunk(
            &note_id,
            0,
            "Budget réunion Q3",
            "Note budget",
            unit_vector(0),
        ),
        make_chunk(
            &note_id,
            1,
            "Appel fournisseur lundi",
            "Note appel",
            unit_vector(1),
        ),
    ];

    store
        .store_chunks(chunks)
        .await
        .expect("store_chunks should succeed");

    let results = store
        .search(unit_vector(0), 50, None)
        .await
        .expect("search should succeed");

    let mine: Vec<_> =
        results.iter().filter(|r| r.note_id == note_id).collect();
    assert!(
        !mine.is_empty(),
        "expected at least one chunk for note_id {note_id}, got {} total results",
        results.len()
    );
    assert!(
        mine.iter().any(|r| r.chunk_text == "Budget réunion Q3"),
        "expected closest chunk to contain the stored text"
    );

    cleanup(&store, &note_id).await;
}

#[tokio::test]
async fn test_vectordb_delete_chunks() {
    let store = ensure_table().await;
    let note_id = Uuid::new_v4().to_string();

    let chunks = vec![make_chunk(
        &note_id,
        0,
        "Texte temporaire",
        "Titre temp",
        unit_vector(2),
    )];

    store.store_chunks(chunks).await.expect("store_chunks");

    let before = store
        .search(unit_vector(2), 50, None)
        .await
        .expect("search before");
    assert!(
        before.iter().any(|r| r.note_id == note_id),
        "chunk should be present before delete"
    );

    store
        .delete_note_chunks(&note_id)
        .await
        .expect("delete_note_chunks");

    let after = store
        .search(unit_vector(2), 50, None)
        .await
        .expect("search after");
    assert!(
        !after.iter().any(|r| r.note_id == note_id),
        "chunk should be gone after delete, leftover: {}",
        after.iter().filter(|r| r.note_id == note_id).count()
    );
}

#[tokio::test]
async fn test_note_and_attachment_chunks_coexist() {
    let store = ensure_table().await;
    let note_id = Uuid::new_v4().to_string();
    let att_id = Uuid::new_v4().to_string();

    let note_chunk = Chunk {
        id: format!("note:{note_id}:0"),
        note_id: note_id.clone(),
        chunk_text: "note body original".to_string(),
        chunk_index: 0,
        vector: unit_vector(10),
        title: "note".to_string(),
        tags: String::new(),
        created_at: "2026-05-11T00:00:00Z".to_string(),
    };
    store
        .store_chunks(vec![note_chunk])
        .await
        .expect("store note");

    let att_chunk = Chunk {
        id: format!("att:{att_id}:0"),
        note_id: note_id.clone(),
        chunk_text: "attachment body".to_string(),
        chunk_index: 0,
        vector: unit_vector(11),
        title: "attachment".to_string(),
        tags: String::new(),
        created_at: "2026-05-11T00:00:00Z".to_string(),
    };
    store
        .store_chunks(vec![att_chunk])
        .await
        .expect("store att");

    let note_hits = store.search(unit_vector(10), 100, None).await.unwrap();
    let att_hits = store.search(unit_vector(11), 100, None).await.unwrap();
    assert!(
        note_hits
            .iter()
            .any(|r| r.chunk_text == "note body original"),
        "note chunk must be present"
    );
    assert!(
        att_hits.iter().any(|r| r.chunk_text == "attachment body"),
        "attachment chunk must be present"
    );

    let note_v2 = Chunk {
        id: format!("note:{note_id}:0"),
        note_id: note_id.clone(),
        chunk_text: "note body edited".to_string(),
        chunk_index: 0,
        vector: unit_vector(10),
        title: "note".to_string(),
        tags: String::new(),
        created_at: "2026-05-11T00:00:00Z".to_string(),
    };
    store
        .store_chunks(vec![note_v2])
        .await
        .expect("re-embed note");

    let att_after = store.search(unit_vector(11), 100, None).await.unwrap();
    assert!(
        att_after.iter().any(|r| r.chunk_text == "attachment body"),
        "attachment chunk must survive a note re-embed (BLOCKER 5 regression)"
    );
    let note_after = store.search(unit_vector(10), 100, None).await.unwrap();
    let mine: Vec<_> = note_after
        .iter()
        .filter(|r| {
            r.note_id == note_id && r.chunk_text.starts_with("note body")
        })
        .collect();
    assert!(
        mine.iter().all(|r| r.chunk_text == "note body edited"),
        "note re-embed must replace the old chunk, no orphan"
    );

    cleanup(&store, &note_id).await;
}

#[tokio::test]
async fn test_vectordb_store_empty_is_noop() {
    isolate_store();
    let store = VectorStore::open().await.expect("open vectordb");
    let result = store.store_chunks(vec![]).await;
    assert!(result.is_ok(), "storing zero chunks should be a no-op");
}

#[tokio::test]
async fn test_vectordb_delete_unknown_note() {
    isolate_store();
    let store = VectorStore::open().await.expect("open vectordb");
    let unknown = Uuid::new_v4().to_string();
    let result = store.delete_note_chunks(&unknown).await;
    assert!(result.is_ok(), "deleting an unknown note_id must not error");
}

#[tokio::test]
async fn test_vectordb_store_upserts_same_note_id() {
    let store = ensure_table().await;
    let note_id = Uuid::new_v4().to_string();

    let first = vec![make_chunk(
        &note_id,
        0,
        "version A",
        "titre",
        unit_vector(3),
    )];
    store.store_chunks(first).await.expect("first store");

    let second = vec![
        make_chunk(&note_id, 0, "version B chunk 0", "titre", unit_vector(3)),
        make_chunk(&note_id, 1, "version B chunk 1", "titre", unit_vector(3)),
    ];
    store.store_chunks(second).await.expect("second store");

    let results = store
        .search(unit_vector(3), 100, None)
        .await
        .expect("search after upsert");

    let mine: Vec<_> =
        results.iter().filter(|r| r.note_id == note_id).collect();
    assert_eq!(
        mine.len(),
        2,
        "after upsert, expected 2 chunks for note, got {} (texts: {:?})",
        mine.len(),
        mine.iter().map(|r| &r.chunk_text).collect::<Vec<_>>()
    );
    assert!(mine.iter().all(|r| r.chunk_text.starts_with("version B")));

    cleanup(&store, &note_id).await;
}

#[tokio::test]
#[ignore]
async fn test_real_embedding() {
    use flowflow::services::llm::LlmClient;

    let client = LlmClient::from_env().expect("OPENAI_API_KEY required");
    let vector = client
        .embed("Hello world from FlowFlow")
        .await
        .expect("embed should succeed");

    assert_eq!(
        vector.len(),
        EMBEDDING_DIMS,
        "embedding dim must match EMBEDDING_DIMS"
    );
    assert!(
        vector.iter().any(|&v| v != 0.0),
        "embedding should not be all zeros"
    );
}

#[tokio::test]
#[ignore]
async fn test_real_chat() {
    use flowflow::services::llm::LlmClient;

    let client = LlmClient::from_env().expect("OPENAI_API_KEY required");
    let response = client
        .chat(
            "You answer in one short word.",
            "Reply with the single word: pong",
        )
        .await
        .expect("chat should succeed");

    assert!(
        !response.trim().is_empty(),
        "chat response must not be empty"
    );
}

#[tokio::test]
#[ignore]
async fn test_real_rag_pipeline() {
    use flowflow::services::llm::LlmClient;

    let client = LlmClient::from_env().expect("OPENAI_API_KEY required");
    let store = ensure_table().await;

    let note_id = Uuid::new_v4().to_string();
    let note_text = "Le rendez-vous chez le dentiste est prévu mardi à 14h30.";
    let vector = client.embed(note_text).await.expect("embed note");

    let chunk =
        make_chunk(&note_id, 0, note_text, "Rendez-vous dentiste", vector);
    store.store_chunks(vec![chunk]).await.expect("store chunk");

    let query_vec = client
        .embed("Quand est mon rendez-vous chez le dentiste ?")
        .await
        .expect("embed query");

    let results = store
        .search(query_vec, 5, None)
        .await
        .expect("search vectordb");

    let mine: Vec<_> =
        results.iter().filter(|r| r.note_id == note_id).collect();
    assert!(
        !mine.is_empty(),
        "RAG search must surface the stored note as a top match"
    );

    cleanup(&store, &note_id).await;
}
