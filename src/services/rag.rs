use crate::db::Database;
use crate::services::constants::{
    DEFAULT_RAG_MAX_SOURCES, RAG_AGENT_SYSTEM_PROMPT, RAG_DISTANCE_THRESHOLD,
    RAG_FINAL_K, RAG_INITIAL_K, RERANK_PROMPT, TEMPORAL_DETECT_PROMPT,
};
use crate::services::llm::LlmClient;
use crate::services::tools::{prompt_agent_with_tools, ToolEvent};
use crate::services::vectordb::{SearchResult, VectorStore};
use chrono::{Datelike, Local, NaiveDate};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct RagSource {
    pub note_id: String,
    pub title: String,
    pub chunk_text: String,
    pub distance: f32,
    pub created_at: String,
}

#[derive(Clone)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<RagSource>,
}

pub fn build_context(results: &[SearchResult]) -> String {
    let mut ctx = String::from("--- User notes ---\n\n");
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

struct DateRange {
    from: NaiveDate,
    to: NaiveDate,
}

fn detect_temporal_regex(question: &str) -> Option<DateRange> {
    let today = Local::now().date_naive();
    let q = question.to_lowercase();

    if q.contains("aujourd'hui") || q.contains("aujourd'hui") {
        return Some(DateRange {
            from: today,
            to: today,
        });
    }
    if q.contains("hier") {
        let d = today - chrono::Duration::days(1);
        return Some(DateRange { from: d, to: d });
    }
    if q.contains("cette semaine") {
        let weekday = today.weekday().num_days_from_monday();
        let monday = today - chrono::Duration::days(weekday as i64);
        return Some(DateRange {
            from: monday,
            to: today,
        });
    }
    if q.contains("semaine dernière") || q.contains("semaine passée") {
        let weekday = today.weekday().num_days_from_monday();
        let this_monday = today - chrono::Duration::days(weekday as i64);
        let last_monday = this_monday - chrono::Duration::days(7);
        let last_sunday = this_monday - chrono::Duration::days(1);
        return Some(DateRange {
            from: last_monday,
            to: last_sunday,
        });
    }
    if q.contains("ce mois") || q.contains("ce mois-ci") {
        let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)?;
        return Some(DateRange {
            from: first,
            to: today,
        });
    }
    if q.contains("mois dernier") || q.contains("mois passé") {
        let first_this =
            NaiveDate::from_ymd_opt(today.year(), today.month(), 1)?;
        let last_day_prev = first_this - chrono::Duration::days(1);
        let first_prev = NaiveDate::from_ymd_opt(
            last_day_prev.year(),
            last_day_prev.month(),
            1,
        )?;
        return Some(DateRange {
            from: first_prev,
            to: last_day_prev,
        });
    }

    let fr_months = [
        ("janvier", 1),
        ("février", 2),
        ("fevrier", 2),
        ("mars", 3),
        ("avril", 4),
        ("mai", 5),
        ("juin", 6),
        ("juillet", 7),
        ("août", 8),
        ("aout", 8),
        ("septembre", 9),
        ("octobre", 10),
        ("novembre", 11),
        ("décembre", 12),
        ("decembre", 12),
    ];
    for (name, month) in fr_months {
        if q.contains(name) {
            let year = today.year();
            let first = NaiveDate::from_ymd_opt(year, month, 1)?;
            let next_month = if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1)?
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1)?
            };
            let last = next_month - chrono::Duration::days(1);
            return Some(DateRange {
                from: first,
                to: last,
            });
        }
    }

    None
}

async fn detect_temporal_llm(
    llm: &LlmClient,
    question: &str,
) -> Option<DateRange> {
    let today = Local::now().date_naive();
    let user_msg = format!("Today: {today}\nQuestion: {question}");
    let response = match llm.chat(TEMPORAL_DETECT_PROMPT, &user_msg).await {
        Ok(r) => r,
        Err(_) => return None,
    };
    let trimmed = response.trim();
    if trimmed == "null" || trimmed.is_empty() {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let from_str = parsed.get("from")?.as_str()?;
    let to_str = parsed.get("to")?.as_str()?;
    let from = NaiveDate::parse_from_str(from_str, "%Y-%m-%d").ok()?;
    let to = NaiveDate::parse_from_str(to_str, "%Y-%m-%d").ok()?;
    Some(DateRange { from, to })
}

fn apply_date_filter(
    results: Vec<SearchResult>,
    range: &DateRange,
) -> Vec<SearchResult> {
    let from_str = range.from.format("%Y-%m-%d").to_string();
    let to_str = range.to.format("%Y-%m-%d").to_string();
    results
        .into_iter()
        .filter(|r| {
            let date_part = if r.created_at.len() >= 10 {
                &r.created_at[..10]
            } else {
                &r.created_at
            };
            date_part >= from_str.as_str() && date_part <= to_str.as_str()
        })
        .collect()
}

pub async fn query(
    question: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
    folder_id: Option<String>,
) -> Result<RagResponse, String> {
    let ai = Arc::new(LlmClient::from_env()?);
    let store = VectorStore::open().await?;

    let allowed_note_ids: Option<Vec<String>> = folder_id.and_then(|fid| {
        Database::open().ok().map(|db| {
            db.list_notes_in_folder(&fid)
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.id)
                .collect()
        })
    });

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

    let mut reranked = llm_rerank(&ai, question, candidates, RAG_FINAL_K).await;
    apply_temporal_boost(&mut reranked);
    let filtered = filter_and_dedup(reranked);

    let user_max = read_max_sources();
    let source_count = compute_source_count(&filtered, user_max);
    let results: Vec<SearchResult> =
        filtered.into_iter().take(source_count).collect();

    let context = if results.is_empty() {
        String::from("--- User notes ---\n\n(no relevant excerpts)\n")
    } else {
        let db_tags = Database::open().ok();
        let mut ctx = String::from("--- User notes ---\n\n");
        for (i, r) in results.iter().enumerate() {
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
        ctx
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
            created_at: r.created_at,
        })
        .collect();

    Ok(RagResponse { answer, sources })
}
