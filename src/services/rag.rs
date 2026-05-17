use crate::db::Database;
use crate::services::constants::{
    DEFAULT_RAG_MAX_SOURCES, RAG_AGENT_SYSTEM_PROMPT, RAG_DISTANCE_THRESHOLD,
    RAG_FINAL_K, RAG_INITIAL_K, RERANK_PROMPT,
};
use crate::services::llm::LlmClient;
use crate::services::tools::{prompt_agent_with_tools, ToolEvent};
use crate::services::vectordb::{SearchResult, VectorStore};
use std::collections::HashSet;
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

fn parse_rerank_indices(response: &str, max: usize) -> Vec<usize> {
    response
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&i| i >= 1 && i <= max)
        .map(|i| i - 1)
        .collect()
}

async fn llm_rerank(
    llm: &LlmClient,
    question: &str,
    results: Vec<SearchResult>,
    final_k: usize,
) -> Vec<SearchResult> {
    if results.len() <= final_k {
        return results;
    }

    let passages: String = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let preview_len = 200.min(r.chunk_text.len());
            format!("[{}] {}: {}", i + 1, r.title, &r.chunk_text[..preview_len])
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let user_msg = format!(
        "Question: {question}\
         \n\nPassages:\n{passages}\
         \n\nReturn top {final_k} indices, most relevant first."
    );

    let response = match llm.chat(RERANK_PROMPT, &user_msg).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[rag] rerank failed, using RRF order: {e}");
            return results.into_iter().take(final_k).collect();
        }
    };

    let indices = parse_rerank_indices(&response, results.len());
    if indices.is_empty() {
        eprintln!("[rag] rerank parse empty, using RRF order");
        return results.into_iter().take(final_k).collect();
    }

    let mut reranked: Vec<SearchResult> = indices
        .into_iter()
        .filter_map(|i| results.get(i).cloned())
        .collect();
    reranked.truncate(final_k);
    reranked
}

fn apply_temporal_boost(results: &mut [SearchResult]) {
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
    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
}

fn filter_and_dedup(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen = HashSet::new();
    results
        .into_iter()
        .filter(|r| r.distance <= RAG_DISTANCE_THRESHOLD)
        .filter(|r| seen.insert(r.note_id.clone()))
        .collect()
}

fn read_max_sources() -> usize {
    Database::open()
        .ok()
        .and_then(|d| d.get_setting("rag_max_sources"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_RAG_MAX_SOURCES)
}

fn compute_source_count(results: &[SearchResult], user_max: usize) -> usize {
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

pub async fn query(
    question: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
) -> Result<RagResponse, String> {
    let ai = Arc::new(LlmClient::from_env()?);
    let store = VectorStore::open().await?;

    let _ = store.ensure_fts_index().await;
    let query_vector = ai.embed(question).await?;
    let candidates = store
        .hybrid_search(question, query_vector, RAG_INITIAL_K)
        .await?;

    let mut reranked = llm_rerank(&ai, question, candidates, RAG_FINAL_K).await;
    apply_temporal_boost(&mut reranked);
    let filtered = filter_and_dedup(reranked);

    let user_max = read_max_sources();
    let source_count = compute_source_count(&filtered, user_max);
    let results: Vec<SearchResult> =
        filtered.into_iter().take(source_count).collect();

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
