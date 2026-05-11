use crate::services::constants::{
    CHAT_MODEL, CHUNK_OVERLAP_WORDS, CHUNK_SIZE_WORDS, EMBEDDING_DIMS,
    EMBEDDING_MODEL, OPENAI_BASE_URL,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

pub struct OpenAIClient {
    client: reqwest::Client,
    api_key: String,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let key = crate::db::Database::open()
            .ok()
            .and_then(|db| db.get_setting("openai_api_key"))
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| option_env!("OPENAI_API_KEY").map(String::from))
            .unwrap_or_default();
        if key.is_empty() || key == "your_key_here" {
            return Err("OPENAI_API_KEY not configured".to_string());
        }
        Ok(Self::new(key))
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        eprintln!("[openai] embedding {} chars", text.len());
        let body = serde_json::json!({
            "model": EMBEDDING_MODEL,
            "input": text,
            "dimensions": EMBEDDING_DIMS
        });
        let resp = self
            .client
            .post(format!("{OPENAI_BASE_URL}/v1/embeddings"))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Embedding request error: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Embedding failed ({status}): {body}"));
        }
        let data: EmbeddingResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse embedding error: {e}"))?;
        data.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| "No embedding in response".to_string())
    }

    pub async fn chat(
        &self,
        system: &str,
        user_message: &str,
    ) -> Result<String, String> {
        eprintln!("[openai] chat request ({} chars)", user_message.len());
        let body = serde_json::json!({
            "model": CHAT_MODEL,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user_message }
            ],
            "temperature": 0.3
        });
        let resp = self
            .client
            .post(format!("{OPENAI_BASE_URL}/v1/chat/completions"))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Chat request error: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Chat failed ({status}): {body}"));
        }
        let data: ChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse chat error: {e}"))?;
        data.choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| "No response content".to_string())
    }

    pub async fn generate_tags(
        &self,
        content: &str,
    ) -> Result<Vec<String>, String> {
        use crate::services::constants::TAGS_SYSTEM_PROMPT;
        let response = self.chat(TAGS_SYSTEM_PROMPT, content).await?;
        let trimmed = response.trim();
        serde_json::from_str::<Vec<String>>(trimmed).or_else(|_| {
            if let Some(start) = trimmed.find('[') {
                if let Some(end) = trimmed.rfind(']') {
                    return serde_json::from_str(&trimmed[start..=end])
                        .map_err(|e| format!("Parse tags: {e}"));
                }
            }
            Err(format!("Invalid tags JSON: {trimmed}"))
        })
    }
}

pub fn chunk_text(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= CHUNK_SIZE_WORDS {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let end = (start + CHUNK_SIZE_WORDS).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end >= words.len() {
            break;
        }
        start = end - CHUNK_OVERLAP_WORDS;
    }
    chunks
}
