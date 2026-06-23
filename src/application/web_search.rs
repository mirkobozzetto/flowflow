use serde::Deserialize;
use std::time::Duration;

const EXA_URL: &str = "https://api.exa.ai/search";

pub fn exa_api_key(
    db: &crate::infrastructure::persistence::Database,
) -> String {
    db.get_setting("exa_api_key")
        .filter(|v| !v.trim().is_empty())
        .or_else(|| std::env::var("EXA_API_KEY").ok())
        .or_else(|| option_env!("EXA_API_KEY").map(String::from))
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct WebResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub published_date: Option<String>,
}

#[derive(Deserialize)]
struct ExaResponse {
    #[serde(default)]
    results: Vec<ExaResult>,
}

#[derive(Deserialize)]
struct ExaResult {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    highlights: Vec<String>,
    #[serde(default, rename = "publishedDate")]
    published_date: Option<String>,
}

pub async fn exa_search(query: &str, api_key: &str) -> Vec<WebResult> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let body = serde_json::json!({
        "query": query,
        "type": "auto",
        "numResults": 5,
        "contents": { "highlights": true }
    });

    let resp = match client
        .post(EXA_URL)
        .header("x-api-key", api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[exa] request failed: {e}");
            return Vec::new();
        }
    };

    if !resp.status().is_success() {
        eprintln!("[exa] http {}", resp.status());
        return Vec::new();
    }

    let parsed: ExaResponse = match resp.json().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[exa] parse failed: {e}");
            return Vec::new();
        }
    };

    parsed
        .results
        .into_iter()
        .map(|r| WebResult {
            title: r.title.unwrap_or_default(),
            url: r.url.unwrap_or_default(),
            snippet: r.highlights.into_iter().next().unwrap_or_default(),
            published_date: r.published_date,
        })
        .collect()
}
