use crate::services::constants::{RAG_AGENT_SYSTEM_PROMPT, RAG_INITIAL_K};
use crate::services::llm::LlmClient;
use crate::services::tools::{prompt_agent_with_tools, ToolEvent};
use crate::services::vectordb::{SearchResult, VectorStore};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct RagSource {
    pub note_id: String,
    pub title: String,
    pub chunk_text: String,
    pub distance: f32,
}

#[derive(Clone)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<RagSource>,
}

pub fn build_context(results: &[SearchResult]) -> String {
    let mut ctx = String::from("--- Notes de l'utilisateur ---\n\n");
    for (i, r) in results.iter().enumerate() {
        ctx.push_str(&format!(
            "[Source {}] Note: \"{}\"\n{}\n\n",
            i + 1,
            r.title,
            r.chunk_text
        ));
    }
    ctx
}

pub async fn query(
    question: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
) -> Result<RagResponse, String> {
    let ai = Arc::new(LlmClient::from_env()?);
    let store = VectorStore::open().await?;

    let _ = store.ensure_fts_index().await;
    let query_vector = ai.embed(question).await?;
    let results = store
        .hybrid_search(question, query_vector, RAG_INITIAL_K)
        .await?;

    let context = if results.is_empty() {
        String::from(
            "--- Notes de l'utilisateur ---\n\n(aucun extrait initial)\n",
        )
    } else {
        build_context(&results)
    };
    let user_msg = format!("{context}\n--- Question ---\n{question}");

    let answer = prompt_agent_with_tools(
        ai,
        RAG_AGENT_SYSTEM_PROMPT,
        &user_msg,
        status_tx,
    )
    .await?;

    let sources = results
        .into_iter()
        .map(|r| RagSource {
            note_id: r.note_id,
            title: r.title,
            chunk_text: r.chunk_text,
            distance: r.distance,
        })
        .collect();

    Ok(RagResponse { answer, sources })
}
