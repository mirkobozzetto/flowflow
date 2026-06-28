use crate::application::constants::{
    RAG_AGENT_SYSTEM_PROMPT, RAG_AGENT_WEB_SYSTEM_PROMPT, RAG_FINAL_K,
    RAG_INITIAL_K, RRF_K, RRF_LOCAL_WEIGHT, RRF_WEB_WEIGHT,
};
use crate::application::tools::{prompt_agent_with_tools, ToolEvent};
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::vectordb::{SearchResult, SourceType, VectorStore};
use std::sync::Arc;
use tokio::sync::mpsc;

mod fusion;
use fusion::dedup_merged;
pub use fusion::rrf_merge;

mod temporal;
use temporal::{apply_date_filter, detect_temporal_llm, detect_temporal_regex};

mod scoring;
use scoring::{apply_temporal_boost, compute_source_count, filter_and_dedup};

mod rerank;
use rerank::llm_rerank;

mod config;
use config::{read_max_sources, web_search_config};

mod context;
pub use context::build_context;

pub use crate::domain::ChatScope;

#[derive(Clone)]
pub struct RagSource {
    pub note_id: String,
    pub title: String,
    pub chunk_text: String,
    pub distance: f32,
    pub created_at: String,
    pub source_type: SourceType,
    pub url: Option<String>,
}

#[derive(Clone)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<RagSource>,
}

pub async fn query(
    question: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
    scope: Option<ChatScope>,
    lang: &str,
) -> Result<RagResponse, String> {
    let ai = Arc::new(LlmClient::from_env()?);
    let store = VectorStore::open().await?;

    let allowed_note_ids: Option<Vec<String>> = match scope {
        Some(ChatScope::Thread(tid)) => Database::open().ok().map(|db| {
            db.list_thread_notes(&tid)
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.id)
                .collect()
        }),
        Some(ChatScope::Folder(fid)) => Database::open().ok().map(|db| {
            db.list_notes_in_folder(&fid)
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.id)
                .collect()
        }),
        None => None,
    };

    if matches!(allowed_note_ids, Some(ref ids) if ids.is_empty()) {
        return Ok(RagResponse {
            answer: crate::application::i18n::t(lang, "chat-empty-scope"),
            sources: vec![],
        });
    }

    let date_range = detect_temporal_regex(question);
    let date_range = match date_range {
        Some(r) => {
            eprintln!("[rag] temporal regex: {} to {}", r.from, r.to);
            Some(r)
        }
        None => {
            let r = detect_temporal_llm(&ai, question).await;
            if let Some(ref r) = r {
                eprintln!("[rag] temporal LLM: {} to {}", r.from, r.to);
            }
            r
        }
    };

    let _ = store.ensure_fts_index().await;
    let query_vector = ai.embed(question).await?;
    let fetch_k = if date_range.is_some() {
        RAG_INITIAL_K * 3
    } else {
        RAG_INITIAL_K
    };
    let (web_enabled, exa_key) = web_search_config();
    let web_on = web_enabled && !exa_key.trim().is_empty();

    let results: Vec<SearchResult> = if web_on {
        if let Some(ref tx) = status_tx {
            let _ = tx.send(ToolEvent::Started("web_search".into()));
        }
        let (local_res, web_res) = tokio::join!(
            store.hybrid_search(
                question,
                query_vector,
                fetch_k,
                allowed_note_ids.as_deref(),
            ),
            crate::application::web_search::exa_search(question, &exa_key),
        );
        if let Some(ref tx) = status_tx {
            let _ = tx.send(ToolEvent::Finished("web_search".into()));
        }
        let local = local_res?;
        let local = if let Some(ref range) = date_range {
            apply_date_filter(local, range)
        } else {
            local
        };
        eprintln!("[rag] web on: {} local, {} web", local.len(), web_res.len());
        let merged =
            rrf_merge(local, web_res, RRF_K, RRF_LOCAL_WEIGHT, RRF_WEB_WEIGHT);
        let reranked = llm_rerank(&ai, question, merged, RAG_FINAL_K).await;
        let filtered = dedup_merged(reranked);
        let count = read_max_sources().min(RAG_FINAL_K).min(filtered.len());
        filtered.into_iter().take(count).collect()
    } else {
        let candidates = store
            .hybrid_search(
                question,
                query_vector,
                fetch_k,
                allowed_note_ids.as_deref(),
            )
            .await?;

        let candidates = if let Some(ref range) = date_range {
            apply_date_filter(candidates, range)
        } else {
            candidates
        };

        let mut reranked =
            llm_rerank(&ai, question, candidates, RAG_FINAL_K).await;
        apply_temporal_boost(&mut reranked);
        let filtered = filter_and_dedup(reranked);

        let user_max = read_max_sources();
        let source_count = compute_source_count(&filtered, user_max);
        filtered.into_iter().take(source_count).collect()
    };

    let context = if results.is_empty() {
        String::from("--- User notes ---\n\n(no relevant excerpts)\n")
    } else {
        let db_tags = Database::open().ok();
        let mut ctx = String::from("--- User notes ---\n\n");
        for (i, r) in results.iter().enumerate() {
            match r.source_type {
                SourceType::Web => {
                    ctx.push_str(&format!(
                        "[Source {}] Web: \"{}\" ({})\n{}\n\n",
                        i + 1,
                        r.title,
                        r.url.as_deref().unwrap_or(""),
                        r.chunk_text
                    ));
                }
                SourceType::Local => {
                    let tags: Vec<String> = db_tags
                        .as_ref()
                        .and_then(|d| d.get_note(&r.note_id).ok().flatten())
                        .map(|n| n.tags)
                        .unwrap_or_default();
                    let tags_str = if tags.is_empty() {
                        String::new()
                    } else {
                        format!(" [Tags: {}]", tags.join(", "))
                    };
                    ctx.push_str(&format!(
                        "[Source {}] Note: \"{}\"{}\n{}\n\n",
                        i + 1,
                        r.title,
                        tags_str,
                        r.chunk_text
                    ));
                }
            }
        }
        ctx
    };
    let user_msg = format!("{context}\n--- Question ---\n{question}");

    let system_prompt = if web_on {
        RAG_AGENT_WEB_SYSTEM_PROMPT
    } else {
        RAG_AGENT_SYSTEM_PROMPT
    };
    let answer =
        prompt_agent_with_tools(ai, system_prompt, &user_msg, status_tx)
            .await?;

    let sources = results
        .into_iter()
        .map(|r| RagSource {
            note_id: r.note_id,
            title: r.title,
            chunk_text: r.chunk_text,
            distance: r.distance,
            created_at: r.created_at,
            source_type: r.source_type,
            url: r.url,
        })
        .collect();

    Ok(RagResponse { answer, sources })
}

/// Run an explicit "lance xxx" message straight through the note-action agent path
/// (NOTE_ACTION_PROMPT + connected tools), bypassing RAG retrieval. The reply is a one-line
/// confirmation with a link, rendered as the same action card as in a note. No notes are
/// retrieved, so the response carries no sources.
pub async fn run_action(
    question: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
) -> Result<RagResponse, String> {
    let ai = Arc::new(LlmClient::from_env()?);
    let answer = prompt_agent_with_tools(
        ai,
        crate::application::constants::NOTE_ACTION_PROMPT,
        question,
        status_tx,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(RagResponse {
        answer,
        sources: vec![],
    })
}
