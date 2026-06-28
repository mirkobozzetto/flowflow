use crate::application::constants::RERANK_PROMPT;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::vectordb::SearchResult;

fn parse_rerank_indices(response: &str, max: usize) -> Vec<usize> {
    response
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&i| i >= 1 && i <= max)
        .map(|i| i - 1)
        .collect()
}

pub(super) async fn llm_rerank(
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
            let preview =
                crate::application::ai::char_prefix(&r.chunk_text, 200);
            format!("[{}] {}: {}", i + 1, r.title, preview)
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
