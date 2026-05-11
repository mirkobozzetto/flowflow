use crate::services::constants::{CHAT_MODEL, EMBEDDING_DIMS, EMBEDDING_MODEL};
use crate::services::error::LlmError;
use rig::client::{CompletionClient, EmbeddingsClient};
use rig::completion::Prompt;
use rig::embeddings::EmbeddingModel;
use rig::providers::openai;

pub struct LlmClient {
    client: openai::Client,
}

impl LlmClient {
    pub fn inner(&self) -> &openai::Client {
        &self.client
    }

    pub fn from_env() -> Result<Self, LlmError> {
        let key = crate::db::Database::open()
            .ok()
            .and_then(|db| db.get_setting("openai_api_key"))
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| option_env!("OPENAI_API_KEY").map(String::from))
            .unwrap_or_default();
        if key.is_empty() || key == "your_key_here" {
            return Err(LlmError::NotConfigured(
                "OPENAI_API_KEY not configured".into(),
            ));
        }
        let client = openai::Client::new(&key)
            .map_err(|e| LlmError::NotConfigured(e.to_string()))?;
        Ok(Self { client })
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        let model = self
            .client
            .embedding_model_with_ndims(EMBEDDING_MODEL, EMBEDDING_DIMS);
        let embedding = model
            .embed_text(text)
            .await
            .map_err(|e| LlmError::Embedding(e.to_string()))?;
        Ok(embedding.vec.iter().map(|v| *v as f32).collect())
    }

    pub async fn chat(
        &self,
        system: &str,
        user_message: &str,
    ) -> Result<String, LlmError> {
        let agent = self
            .client
            .agent(CHAT_MODEL)
            .preamble(system)
            .temperature(0.3)
            .build();
        agent
            .prompt(user_message)
            .await
            .map_err(|e| LlmError::Completion(e.to_string()))
    }

    pub async fn generate_tags(
        &self,
        content: &str,
    ) -> Result<Vec<String>, LlmError> {
        use crate::services::constants::TAGS_SYSTEM_PROMPT;
        let response = self.chat(TAGS_SYSTEM_PROMPT, content).await?;
        parse_tags(&response)
    }
}

pub fn parse_tags(response: &str) -> Result<Vec<String>, LlmError> {
    let trimmed = response.trim();
    serde_json::from_str::<Vec<String>>(trimmed).or_else(|_| {
        if let Some(start) = trimmed.find('[') {
            if let Some(end) = trimmed.rfind(']') {
                return serde_json::from_str(&trimmed[start..=end]).map_err(
                    |e| LlmError::TagParsing(format!("Parse tags: {e}")),
                );
            }
        }
        Err(LlmError::TagParsing(format!(
            "Invalid tags JSON: {trimmed}"
        )))
    })
}
