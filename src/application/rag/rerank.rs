use crate::application::constants::{CHEAP_MODEL, RELEVANCE_FILTER_PROMPT};
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::vectordb::SearchResult;
use std::collections::HashSet;

fn parse_relevant_indices(response: &str, max: usize) -> Vec<usize> {
    let mut seen = HashSet::new();
    response
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse::<usize>().ok())
        .filter(|&i| i >= 1 && i <= max)
        .map(|i| i - 1)
        .filter(|i| seen.insert(*i))
        .collect()
}

/// Ask the LLM to judge WHICH retrieved passages are genuinely relevant to the question, and
/// return their indices (0-based, most relevant first, deduped, capped at `max_k`). An empty
/// result means the judge found NOTHING relevant - that drives the abstain decision. A fixed
/// cosine threshold cannot separate relevant from noise on a small personal corpus (the scores
/// cluster), so relevance is decided by the model that actually understands the question.
///
/// This ALWAYS calls the model (no early return on small inputs) because the small-input case is
/// exactly where the old reranker silently skipped, leaving every neighbour cited.
/// On an LLM/transport error it falls back to keeping the top `max_k` (never abstain on infra
/// failure).
pub(super) async fn llm_relevance_filter(
    llm: &LlmClient,
    question: &str,
    results: &[SearchResult],
    max_k: usize,
) -> Vec<usize> {
    if results.is_empty() {
        return Vec::new();
    }

    let passages: String = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let preview =
                crate::application::ai::char_prefix(&r.chunk_text, 200);
            format!("[{}] {}: {}", i + 1, r.title, preview)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let user_msg = format!("Question: {question}\n\nPassages:\n{passages}");

    // The judge reads short previews and returns indices: the cheap tier handles that,
    // and this call sits on the serial path of every question.
    let response = match llm
        .chat_with_model(CHEAP_MODEL, RELEVANCE_FILTER_PROMPT, &user_msg)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "[rag] relevance filter failed, keeping top {max_k}: {e}"
            );
            return (0..results.len().min(max_k)).collect();
        }
    };

    let mut idx = parse_relevant_indices(&response, results.len());
    idx.truncate(max_k);
    idx
}
