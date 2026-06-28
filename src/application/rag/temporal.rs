use crate::application::constants::TEMPORAL_DETECT_PROMPT;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::vectordb::SearchResult;
use chrono::{Datelike, Local, NaiveDate};

pub(super) struct DateRange {
    pub(super) from: NaiveDate,
    pub(super) to: NaiveDate,
}

pub(super) fn detect_temporal_regex(question: &str) -> Option<DateRange> {
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

pub(super) async fn detect_temporal_llm(
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

pub(super) fn apply_date_filter(
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
