//! One prompt, no tools, on the model set for chapters and summaries,
//! whatever the chat runs on.
use super::*;
use crate::application::constants::{
    ANTHROPIC_HAIKU_MODEL, READING_CHATGPT_EFFORT, READING_CHATGPT_MODEL,
    READING_OPENAI_MODEL,
};

impl LlmClient {
    /// The model `chat_reading` asks, kept with what it made.
    pub fn reading_model(&self) -> &'static str {
        match self.provider {
            Provider::OpenAi => READING_OPENAI_MODEL,
            Provider::ChatGpt => READING_CHATGPT_MODEL,
            _ => ANTHROPIC_HAIKU_MODEL,
        }
    }

    pub async fn chat_reading(
        &self,
        system: &str,
        user_message: &str,
    ) -> Result<String, LlmError> {
        let reply = match self.provider {
            Provider::OpenAi => {
                let client = self.openai.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(crate::application::i18n::t(
                        &self.lang,
                        "llm-error-openai-key",
                    ))
                })?;
                // The GPT-6 models take no temperature.
                client
                    .agent(READING_OPENAI_MODEL)
                    .preamble(system)
                    .build()
                    .prompt(user_message)
                    .await
            }
            Provider::Anthropic => {
                let client = self.anthropic.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(
                        "Anthropic client not initialized".into(),
                    )
                })?;
                client
                    .agent(ANTHROPIC_HAIKU_MODEL)
                    .preamble(system)
                    .max_tokens(ANTHROPIC_MAX_TOKENS)
                    .build()
                    .prompt(user_message)
                    .await
            }
            Provider::ChatGpt => {
                self.chatgpt_client()
                    .await?
                    .agent(READING_CHATGPT_MODEL)
                    .preamble(system)
                    .additional_params(serde_json::json!({
                        "reasoning": { "effort": READING_CHATGPT_EFFORT }
                    }))
                    .build()
                    .prompt(user_message)
                    .await
            }
            #[cfg(debug_assertions)]
            Provider::ClaudeSubscription => {
                let reply = self
                    .claude_subscription_client()
                    .await?
                    .agent(ANTHROPIC_HAIKU_MODEL)
                    .preamble(system)
                    .max_tokens(ANTHROPIC_MAX_TOKENS)
                    .build()
                    .prompt(user_message)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()));
                return self.claude_subscription_reply(reply);
            }
        };
        reply.map_err(|e| LlmError::Completion(e.to_string()))
    }
}
