use crate::application::constants::{
    CHEAP_MODEL, EMBEDDING_MODEL, RAG_AGENT_SYSTEM_PROMPT,
    RAG_AGENT_WEB_SYSTEM_PROMPT, RAG_FINAL_K, RAG_INITIAL_K,
    RAG_RELEVANCE_FLOOR, RRF_K, RRF_LOCAL_WEIGHT, RRF_WEB_WEIGHT,
};
use crate::application::tools::{prompt_agent_with_tools, ToolEvent};
use crate::infrastructure::llm::{LlmClient, NotesTools};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::vectordb::{SearchResult, SourceType, VectorStore};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;

mod fusion;
use fusion::dedup_merged;
pub use fusion::rrf_merge;

mod temporal;
use temporal::detect_temporal_llm;
pub use temporal::{apply_date_filter, detect_temporal_regex, DateRange};

/// Apply a date range, but never let it be the reason nothing is answered.
///
/// The filter drops every note outside the period, so a period the user has no
/// notes in turns a good candidate set into an empty one and the chat abstains -
/// looking as if it found nothing at all. When that happens the unfiltered set is
/// kept and the caller is told, so the answer can say the period was empty
/// instead of passing off-period notes as on-period ones.
fn filter_by_date(
    candidates: Vec<crate::infrastructure::vectordb::SearchResult>,
    range: Option<&DateRange>,
) -> (Vec<crate::infrastructure::vectordb::SearchResult>, bool) {
    let Some(range) = range else {
        return (candidates, false);
    };
    if candidates.is_empty() {
        return (candidates, false);
    }
    let filtered = apply_date_filter(candidates.clone(), range);
    if filtered.is_empty() {
        eprintln!("[rag] date filter emptied the set, falling back unfiltered");
        return (candidates, true);
    }
    (filtered, false)
}

mod scoring;
use scoring::{apply_temporal_boost, compute_source_count, dedup_sources};
pub use scoring::{floor_filter, passes_floor};

/// Outcome of the grounded-abstain gate for the notes-only (web-off) path.
#[derive(Debug, PartialEq, Eq)]
pub enum AbstainDecision {
    Abstain,
    Proceed,
}

/// Decide whether to abstain instead of answering. Abstain ONLY when retrieval cleared nothing,
/// web is off, and the user expressed no explicit intent (no `@mention`, not an action request) -
/// otherwise proceed. Pure and total over the matrix, so it is unit-tested without an embed call.
pub fn abstain_decision(
    kept_empty: bool,
    web_on: bool,
    has_mention: bool,
    is_actionable: bool,
) -> AbstainDecision {
    if kept_empty && !web_on && !has_mention && !is_actionable {
        AbstainDecision::Abstain
    } else {
        AbstainDecision::Proceed
    }
}

mod rerank;
use rerank::llm_relevance_filter;

mod trace;
use trace::StepTimer;
pub use trace::{estimate_tokens, RagTrace, StepOutcome, TraceStep};

mod rewrite;
use rewrite::prep_query;
pub use rewrite::{
    build_rewrite_input, format_history, parse_prep, recent_turns,
    sanitize_rewrite, ChatTurn,
};

/// Keep only the candidates the LLM judged genuinely relevant to the question. The model reads
/// the question and the note contents, so it understands "the note about Jean" means the note that
/// discusses Jean - not every note that happens to contain the common word "note". Returns the
/// relevant subset; empty means nothing is relevant (drives the abstain decision).
async fn select_relevant(
    ai: &LlmClient,
    question: &str,
    candidates: Vec<SearchResult>,
    rag_trace: &mut RagTrace,
) -> Vec<SearchResult> {
    if candidates.is_empty() {
        StepTimer::start("judge", None).finish(
            rag_trace,
            None,
            None,
            StepOutcome::Skipped,
        );
        return candidates;
    }
    let judge_in = estimate_tokens(question)
        + candidates
            .iter()
            .map(|c| estimate_tokens(&c.chunk_text))
            .sum::<u32>();
    let timer = StepTimer::start("judge", Some(CHEAP_MODEL));
    let judged: HashSet<usize> =
        llm_relevance_filter(ai, question, &candidates, RAG_FINAL_K)
            .await
            .into_iter()
            .collect();
    timer.finish(rag_trace, Some(judge_in), None, StepOutcome::Ok);
    candidates
        .into_iter()
        .enumerate()
        .filter(|(i, _)| judged.contains(i))
        .map(|(_, r)| r)
        .collect()
}

mod config;
use config::{exa_key, read_max_sources};
pub use config::{resolve_allowed_ids, resolve_web_on};

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
    /// Per-step execution trace (observation only); None on non-RAG paths.
    pub trace: Option<RagTrace>,
}

pub async fn query(
    question: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
    scope: Option<ChatScope>,
    web: bool,
    mention_note_ids: Vec<String>,
    history: Vec<ChatTurn>,
    lang: &str,
) -> Result<RagResponse, String> {
    let run_started = std::time::Instant::now();
    let mut rag_trace = RagTrace::default();
    let ai = Arc::new(LlmClient::from_env()?);
    let store = VectorStore::open().await?;

    let scope_ids: Option<Vec<String>> = match scope {
        Some(ChatScope::Thread(tid)) => Database::open().ok().map(|db| {
            db.list_thread_notes(&tid)
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.id)
                .collect()
        }),
        Some(ChatScope::Folder(fid)) => Database::open().ok().map(|db| {
            db.list_notes_in_folder_tree(&fid)
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.id)
                .collect()
        }),
        None => None,
    };
    // Explicit user intent bypasses the relevance floor: an `@mention` points at named notes,
    // an action request must reach the tools (create/summarize) even with no retrieval hit.
    let has_mention = !mention_note_ids.is_empty();
    let is_actionable = crate::application::intent::is_actionable(question);
    let allowed_note_ids = resolve_allowed_ids(scope_ids, mention_note_ids);

    let exa = exa_key();
    let web_on = resolve_web_on(web, &exa);

    // An empty scope only dead-ends when there is no web to fall back on; with web
    // on, the chat still answers from the web even if the scoped note set is empty.
    if !web_on && matches!(allowed_note_ids, Some(ref ids) if ids.is_empty()) {
        rag_trace.total_ms = run_started.elapsed().as_millis() as u64;
        return Ok(RagResponse {
            answer: crate::application::i18n::t(lang, "chat-empty-scope"),
            sources: vec![],
            trace: Some(rag_trace),
        });
    }

    // Follow-ups lean on the conversation ("et lui ?", "développe"): ONE cheap prep call
    // yields both the standalone retrieval query and the temporal intent, while the final
    // agent still sees the user's words. First messages skip it entirely.
    let (retrieval_q, mut date_range) = if history.is_empty() {
        StepTimer::start("prep", None).finish(
            &mut rag_trace,
            None,
            None,
            StepOutcome::Skipped,
        );
        (question.to_string(), None)
    } else {
        let prep_in = estimate_tokens(question)
            + history
                .iter()
                .map(|t| estimate_tokens(&t.content))
                .sum::<u32>();
        let timer = StepTimer::start("prep", Some(CHEAP_MODEL));
        let (q, range) = prep_query(&ai, &history, question).await;
        timer.finish(
            &mut rag_trace,
            Some(prep_in),
            Some(estimate_tokens(&q)),
            StepOutcome::Ok,
        );
        (q, range)
    };
    if retrieval_q != question {
        eprintln!("[rag] rewrite: {retrieval_q}");
    }
    let retrieval_q = retrieval_q.as_str();

    if date_range.is_none() {
        date_range = detect_temporal_regex(retrieval_q);
        if let Some(ref r) = date_range {
            eprintln!("[rag] temporal regex: {} to {}", r.from, r.to);
        }
    }

    let _ = store.ensure_fts_index().await;

    // The LLM date fallback only fires on a first message with no regex hit (a follow-up's
    // prep call already judged dates). It used to serialize ~2.5s before embed; now the two
    // run concurrently and the wall clock pays only the slower of them. The date filter
    // applies after search and fetch_k is decided after this join, so nothing downstream
    // needs the range earlier.
    let needs_temporal_llm = date_range.is_none() && history.is_empty();
    let temporal_fut = async {
        if !needs_temporal_llm {
            return (None, 0u64);
        }
        let started = std::time::Instant::now();
        let r = detect_temporal_llm(&ai, retrieval_q).await;
        (r, started.elapsed().as_millis() as u64)
    };
    let embed_fut = async {
        let started = std::time::Instant::now();
        let r = ai.embed(retrieval_q).await;
        (r, started.elapsed().as_millis() as u64)
    };
    let ((llm_range, temporal_ms), (embed_res, embed_ms)) =
        tokio::join!(temporal_fut, embed_fut);
    rag_trace.push(TraceStep {
        step: "temporal".to_string(),
        model: needs_temporal_llm.then(|| CHEAP_MODEL.to_string()),
        tokens_in: needs_temporal_llm.then(|| estimate_tokens(retrieval_q)),
        tokens_out: None,
        approx: true,
        latency_ms: temporal_ms,
        retries: 0,
        outcome: if needs_temporal_llm {
            StepOutcome::Ok
        } else {
            StepOutcome::Skipped
        },
    });
    rag_trace.push(TraceStep {
        step: "embed".to_string(),
        model: Some(EMBEDDING_MODEL.to_string()),
        tokens_in: Some(estimate_tokens(retrieval_q)),
        tokens_out: None,
        approx: true,
        latency_ms: embed_ms,
        retries: 0,
        outcome: StepOutcome::Ok,
    });
    let query_vector = embed_res?;
    if date_range.is_none() {
        date_range = llm_range;
        if let Some(ref r) = date_range {
            eprintln!("[rag] temporal LLM: {} to {}", r.from, r.to);
        }
    }
    let fetch_k = if date_range.is_some() {
        RAG_INITIAL_K * 3
    } else {
        RAG_INITIAL_K
    };

    // Set when a date range matched nothing and was waived, so the answer can say
    // the period was empty rather than present off-period notes as on-period.
    let mut period_empty = false;
    let results: Vec<SearchResult> = if web_on {
        if let Some(ref tx) = status_tx {
            let _ = tx.send(ToolEvent::Started("web_search".into()));
        }
        let search_timer = StepTimer::start("search", None);
        let (local_res, web_res) = tokio::join!(
            store.hybrid_search(
                retrieval_q,
                query_vector,
                fetch_k,
                allowed_note_ids.as_deref(),
            ),
            crate::application::web_search::exa_search(retrieval_q, &exa),
        );
        search_timer.finish(&mut rag_trace, None, None, StepOutcome::Ok);
        if let Some(ref tx) = status_tx {
            let _ = tx.send(ToolEvent::Finished("web_search".into()));
        }
        let local = local_res?;
        let (local, empty_period) = filter_by_date(local, date_range.as_ref());
        period_empty = empty_period;
        eprintln!("[rag] web on: {} local, {} web", local.len(), web_res.len());
        // Both legs empty (no local chunk clears the floor AND the web returned nothing) ->
        // say so instead of fabricating from "(no relevant excerpts)" under the web prompt.
        // Explicit intent (mention/action) still proceeds.
        if web_res.is_empty()
            && floor_filter(&local, RAG_RELEVANCE_FLOOR).is_empty()
            && !has_mention
            && !is_actionable
        {
            eprintln!("[rag] abstain: web on, both legs empty");
            rag_trace.total_ms = run_started.elapsed().as_millis() as u64;
            return Ok(RagResponse {
                answer: crate::application::i18n::t(lang, "chat-web-empty"),
                sources: vec![],
                trace: Some(rag_trace),
            });
        }
        let merged =
            rrf_merge(local, web_res, RRF_K, RRF_LOCAL_WEIGHT, RRF_WEB_WEIGHT);
        // Cite only the genuinely relevant rows (notes AND web results), judged by the LLM, not
        // the top-N by rank.
        let selected =
            select_relevant(&ai, retrieval_q, merged, &mut rag_trace).await;
        let filtered = dedup_merged(selected);
        let count = read_max_sources().min(RAG_FINAL_K).min(filtered.len());
        filtered.into_iter().take(count).collect()
    } else {
        let search_timer = StepTimer::start("search", None);
        let candidates = store
            .hybrid_search(
                retrieval_q,
                query_vector,
                fetch_k,
                allowed_note_ids.as_deref(),
            )
            .await?;
        search_timer.finish(&mut rag_trace, None, None, StepOutcome::Ok);

        let (candidates, empty_period) =
            filter_by_date(candidates, date_range.as_ref());
        period_empty = empty_period;

        // The LLM judge decides WHICH notes genuinely answer the question - it reads the content,
        // so it judges what a note is ABOUT, not mere word overlap. An `@mention` is an explicit
        // pointer: answer from the named notes without judging.
        let mut selected = if has_mention {
            StepTimer::start("judge", None).finish(
                &mut rag_trace,
                None,
                None,
                StepOutcome::Skipped,
            );
            candidates
        } else {
            select_relevant(&ai, retrieval_q, candidates, &mut rag_trace).await
        };
        apply_temporal_boost(&mut selected);
        let kept = dedup_sources(selected);
        eprintln!("[rag] cited {} notes", kept.len());

        // Nothing relevant, web off, no explicit intent -> say so and cite no sources.
        if abstain_decision(kept.is_empty(), web_on, has_mention, is_actionable)
            == AbstainDecision::Abstain
        {
            eprintln!("[rag] abstain: nothing relevant (web off)");
            rag_trace.total_ms = run_started.elapsed().as_millis() as u64;
            return Ok(RagResponse {
                answer: crate::application::i18n::t(lang, "chat-no-relevant"),
                sources: vec![],
                trace: Some(rag_trace),
            });
        }

        let user_max = read_max_sources();
        let source_count = compute_source_count(&kept, user_max);
        kept.into_iter().take(source_count).collect()
    };

    // The user asked for a period that holds nothing. Saying so beats both staying
    // silent and letting off-period notes pass for on-period ones.
    let period_note = if period_empty {
        "Note: no note falls in the period the question asks about. The excerpts \
         below are the closest matches from other dates - say the period is empty \
         before using them.\n\n"
    } else {
        ""
    };
    let context = if results.is_empty() {
        String::from("--- User notes ---\n\n(no relevant excerpts)\n")
    } else {
        let db_tags = Database::open().ok();
        let mut ctx = format!("--- User notes ---\n\n{period_note}");
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
    // The agent gets the recent turns verbatim so "développe" stays answerable even
    // when retrieval adds nothing new; the raw question keeps the user's exact words.
    let user_msg = if history.is_empty() {
        format!("{context}\n--- Question ---\n{question}")
    } else {
        format!(
            "{context}\n--- Conversation so far ---\n{}\n\n--- Question ---\n{question}",
            format_history(recent_turns(&history))
        )
    };

    let system_prompt = if web_on {
        RAG_AGENT_WEB_SYSTEM_PROMPT
    } else {
        RAG_AGENT_SYSTEM_PROMPT
    };
    let agent_timer = StepTimer::start("agent", Some(ai.chat_model_name()));
    // Same perimeter as the initial retrieval: inside a folder or a thread, the
    // agent's own re-search must not reach past what the user narrowed to.
    let answer = prompt_agent_with_tools(
        ai,
        system_prompt,
        &user_msg,
        status_tx,
        NotesTools::from_allowed(allowed_note_ids.as_deref()),
    )
    .await?;
    agent_timer.finish(
        &mut rag_trace,
        Some(estimate_tokens(&user_msg)),
        Some(estimate_tokens(&answer)),
        StepOutcome::Ok,
    );

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

    rag_trace.total_ms = run_started.elapsed().as_millis() as u64;
    Ok(RagResponse {
        answer,
        sources,
        trace: Some(rag_trace),
    })
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
    // A note action is global by construction: it is triggered from one note and
    // may create or summarize anywhere.
    let answer = prompt_agent_with_tools(
        ai,
        crate::application::constants::NOTE_ACTION_PROMPT,
        question,
        status_tx,
        NotesTools::Global,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(RagResponse {
        answer,
        sources: vec![],
        trace: None,
    })
}
