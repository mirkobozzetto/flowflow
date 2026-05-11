use crate::services::constants::{
    ANTHROPIC_CHAT_MODEL, ANTHROPIC_MAX_TOKENS, CHAT_MODEL, EMBEDDING_DIMS,
    EMBEDDING_MODEL,
};
use crate::services::error::LlmError;
use crate::services::tools::{
    CreateNote, SearchNotes, SummarizeFolder, ToolEvent, ToolStatusHook,
};
use rig::client::{CompletionClient, EmbeddingsClient};
use rig::completion::Prompt;
use rig::embeddings::EmbeddingModel;
use rig::providers::{anthropic, openai};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Provider {
    #[default]
    OpenAi,
    Anthropic,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::OpenAi => "openai",
            Provider::Anthropic => "anthropic",
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Provider {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "openai" | "open_ai" | "open-ai" => Ok(Provider::OpenAi),
            "anthropic" | "claude" => Ok(Provider::Anthropic),
            _ => Err(()),
        }
    }
}

pub struct LlmClient {
    openai: openai::Client,
    provider: Provider,
    anthropic: Option<anthropic::Client>,
}

impl LlmClient {
    pub fn provider(&self) -> Provider {
        self.provider
    }

    pub fn from_env() -> Result<Self, LlmError> {
        let db = crate::db::Database::open().ok();
        let openai_key = db
            .as_ref()
            .and_then(|d| d.get_setting("openai_api_key"))
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| option_env!("OPENAI_API_KEY").map(String::from))
            .unwrap_or_default();
        if openai_key.is_empty() || openai_key == "your_key_here" {
            return Err(LlmError::NotConfigured(
                "OPENAI_API_KEY not configured".into(),
            ));
        }
        let openai = openai::Client::new(&openai_key)
            .map_err(|e| LlmError::NotConfigured(e.to_string()))?;

        let provider = db
            .as_ref()
            .and_then(|d| d.get_setting("llm_provider"))
            .and_then(|v| Provider::from_str(&v).ok())
            .unwrap_or_default();

        let anthropic = if provider == Provider::Anthropic {
            let key = db
                .as_ref()
                .and_then(|d| d.get_setting("anthropic_api_key"))
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .or_else(|| option_env!("ANTHROPIC_API_KEY").map(String::from))
                .unwrap_or_default();
            if key.is_empty() || key == "your_key_here" {
                return Err(LlmError::NotConfigured(
                    "ANTHROPIC_API_KEY not configured".into(),
                ));
            }
            Some(
                anthropic::Client::new(&key)
                    .map_err(|e| LlmError::NotConfigured(e.to_string()))?,
            )
        } else {
            None
        };

        Ok(Self {
            openai,
            provider,
            anthropic,
        })
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        let model = self
            .openai
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
        match self.provider {
            Provider::OpenAi => {
                let agent = self
                    .openai
                    .agent(CHAT_MODEL)
                    .preamble(system)
                    .temperature(0.3)
                    .build();
                agent
                    .prompt(user_message)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()))
            }
            Provider::Anthropic => {
                let client = self.anthropic.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(
                        "Anthropic client not initialized".into(),
                    )
                })?;
                let agent = client
                    .agent(ANTHROPIC_CHAT_MODEL)
                    .preamble(system)
                    .temperature(0.3)
                    .max_tokens(ANTHROPIC_MAX_TOKENS)
                    .build();
                agent
                    .prompt(user_message)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()))
            }
        }
    }

    pub async fn generate_tags(
        &self,
        content: &str,
    ) -> Result<Vec<String>, LlmError> {
        use crate::services::constants::TAGS_SYSTEM_PROMPT;
        let response = self.chat(TAGS_SYSTEM_PROMPT, content).await?;
        parse_tags(&response)
    }

    pub async fn prompt_with_agent(
        self: &Arc<Self>,
        preamble: &str,
        user_message: &str,
        status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
    ) -> Result<String, LlmError> {
        match self.provider {
            Provider::OpenAi => {
                let agent = self
                    .openai
                    .agent(CHAT_MODEL)
                    .preamble(preamble)
                    .temperature(0.3)
                    .tool(SearchNotes::new(self.clone()))
                    .tool(CreateNote::new())
                    .tool(SummarizeFolder::new(self.clone()))
                    .build();
                let request = agent.prompt(user_message).max_turns(4);
                let result = if let Some(tx) = status_tx {
                    request.with_hook(ToolStatusHook::new(tx)).await
                } else {
                    request.await
                };
                result.map_err(|e| LlmError::Completion(e.to_string()))
            }
            Provider::Anthropic => {
                let client = self.anthropic.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(
                        "Anthropic client not initialized".into(),
                    )
                })?;
                let agent = client
                    .agent(ANTHROPIC_CHAT_MODEL)
                    .preamble(preamble)
                    .temperature(0.3)
                    .max_tokens(ANTHROPIC_MAX_TOKENS)
                    .tool(SearchNotes::new(self.clone()))
                    .tool(CreateNote::new())
                    .tool(SummarizeFolder::new(self.clone()))
                    .build();
                let request = agent.prompt(user_message).max_turns(4);
                let result = if let Some(tx) = status_tx {
                    request.with_hook(ToolStatusHook::new(tx)).await
                } else {
                    request.await
                };
                result.map_err(|e| LlmError::Completion(e.to_string()))
            }
        }
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
