use crate::application::constants::{
    ANTHROPIC_CHAT_MODEL, ANTHROPIC_MAX_TOKENS, CHAT_MODEL, EMBEDDING_DIMS,
    EMBEDDING_MODEL,
};
use crate::application::error::LlmError;
use crate::application::tools::{
    CreateNote, SearchNotes, SummarizeFolder, ToolEvent, ToolStatusHook,
};
use crate::domain::ReminderIntent;
use chrono::{DateTime, Local};
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
        let db = crate::infrastructure::persistence::Database::open()
            .map_err(LlmError::NotConfigured)?;
        Self::from_db(&db)
    }

    pub fn from_db(
        db: &crate::infrastructure::persistence::Database,
    ) -> Result<Self, LlmError> {
        let lang = crate::application::i18n::ui_lang(db);
        if db.get_setting("ai_consent") != Some("true".to_string()) {
            return Err(LlmError::NotConfigured(crate::application::i18n::t(
                &lang,
                "error-ai-consent",
            )));
        }
        let openai_key = db
            .get_setting("openai_api_key")
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| option_env!("OPENAI_API_KEY").map(String::from))
            .unwrap_or_default();
        if openai_key.is_empty() || openai_key == "your_key_here" {
            return Err(LlmError::NotConfigured(crate::application::i18n::t(
                &lang,
                "llm-error-openai-key",
            )));
        }
        let openai = openai::Client::new(&openai_key)
            .map_err(|e| LlmError::NotConfigured(e.to_string()))?;

        let provider = db
            .get_setting("llm_provider")
            .and_then(|v| Provider::from_str(&v).ok())
            .unwrap_or_default();

        let anthropic = if provider == Provider::Anthropic {
            let key = db
                .get_setting("anthropic_api_key")
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
        use crate::application::constants::TAGS_SYSTEM_PROMPT;
        let response = self.chat(TAGS_SYSTEM_PROMPT, content).await?;
        parse_tags(&response)
    }

    pub async fn generate_title(
        &self,
        content: &str,
        lang: &str,
    ) -> Result<String, LlmError> {
        use crate::application::constants::TITLE_SYSTEM_PROMPT;
        let lang_name = if lang == "fr" { "French" } else { "English" };
        let system =
            format!("{TITLE_SYSTEM_PROMPT}\n\nRespond ONLY in {lang_name}.");
        let preview: String = content.chars().take(1500).collect();
        let response = self.chat(&system, &preview).await?;
        let title = response.trim().trim_matches('"').trim().to_string();
        if title.is_empty() {
            return Err(LlmError::Completion("Empty title".into()));
        }
        Ok(title)
    }

    pub async fn extract_reminders(
        &self,
        text: &str,
        now: DateTime<Local>,
    ) -> Result<Vec<ReminderIntent>, LlmError> {
        use crate::application::constants::REMINDER_EXTRACTION_PROMPT;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let preview: String = trimmed.chars().take(4000).collect();
        let system = format!(
            "{REMINDER_EXTRACTION_PROMPT}\n\nCurrent date and time: {}.",
            now.format("%Y-%m-%d %H:%M (%A)")
        );
        let response = self.chat(&system, &preview).await?;
        let intents = parse_reminder_intents(&response)?;
        Ok(intents.into_iter().filter(|i| i.has_date()).collect())
    }

    pub async fn prompt_with_agent(
        self: &Arc<Self>,
        preamble: &str,
        user_message: &str,
        status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
    ) -> Result<String, LlmError> {
        // Connector tools are dark unless a backend is configured AND reachable. The
        // registry is held in scope for the whole prompt so the tools' server sink stays
        // valid; an empty/failed registry leaves exactly the three notes tools (finding 32).
        let mcp = self.connect_mcp().await;
        let mcp = mcp.as_ref().filter(|r| !r.is_empty());

        match self.provider {
            Provider::OpenAi => {
                let mut builder = self
                    .openai
                    .agent(CHAT_MODEL)
                    .preamble(preamble)
                    .temperature(0.3)
                    .tool(SearchNotes::new(self.clone()))
                    .tool(CreateNote::new())
                    .tool(SummarizeFolder::new(self.clone()));
                if let Some(reg) = mcp {
                    builder = builder.rmcp_tools(reg.tools(), reg.peer());
                }
                let agent = builder.build();
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
                let mut builder = client
                    .agent(ANTHROPIC_CHAT_MODEL)
                    .preamble(preamble)
                    .temperature(0.3)
                    .max_tokens(ANTHROPIC_MAX_TOKENS)
                    .tool(SearchNotes::new(self.clone()))
                    .tool(CreateNote::new())
                    .tool(SummarizeFolder::new(self.clone()));
                if let Some(reg) = mcp {
                    builder = builder.rmcp_tools(reg.tools(), reg.peer());
                }
                let agent = builder.build();
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

    /// Connect to the backend MCP proxy if one is configured. Returns None (notes-only
    /// agent, identical to pre-RFC behavior) when no backend is set or the connect fails.
    async fn connect_mcp(
        &self,
    ) -> Option<crate::infrastructure::mcp::McpRegistry> {
        let db = crate::infrastructure::persistence::Database::open().ok()?;
        let backend =
            crate::infrastructure::backend::BackendClient::from_db(&db)?;
        match crate::infrastructure::mcp::McpRegistry::connect(&db, &backend)
            .await
        {
            Ok(reg) => Some(reg),
            Err(e) => {
                eprintln!("MCP connect failed, notes tools only: {e}");
                None
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

pub fn parse_reminder_intents(
    response: &str,
) -> Result<Vec<ReminderIntent>, LlmError> {
    #[derive(serde::Deserialize)]
    struct Envelope {
        #[serde(default)]
        intents: Vec<ReminderIntent>,
    }
    let trimmed = response.trim();
    let json = extract_json_object(trimmed).unwrap_or(trimmed);
    serde_json::from_str::<Envelope>(json)
        .map(|e| e.intents)
        .map_err(|e| {
            LlmError::ReminderParsing(format!("Invalid reminders JSON: {e}"))
        })
}

fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DEFAULT_REMINDER_HOUR;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn parses_single_intent_with_time() {
        let raw = r#"{"intents":[{"action":"appeler Paul","date":"2026-06-02","time":"15:00","recurrence":null,"location":null}]}"#;
        let v = parse_reminder_intents(raw).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].action, "appeler Paul");
        assert_eq!(v[0].resolved_date(), NaiveDate::from_ymd_opt(2026, 6, 2));
        assert_eq!(
            v[0].resolved_time(),
            NaiveTime::from_hms_opt(15, 0, 0).unwrap()
        );
        assert!(v[0].has_explicit_time());
        assert!(v[0].has_date());
    }

    #[test]
    fn defaults_missing_time_to_nine() {
        let raw = r#"{"intents":[{"action":"faire les courses","date":"2026-06-06","time":null}]}"#;
        let v = parse_reminder_intents(raw).unwrap();
        assert_eq!(v.len(), 1);
        assert!(!v[0].has_explicit_time());
        assert_eq!(
            v[0].resolved_time(),
            NaiveTime::from_hms_opt(DEFAULT_REMINDER_HOUR, 0, 0).unwrap()
        );
    }

    #[test]
    fn parses_multiple_intents_with_recurrence() {
        let raw = r#"{"intents":[
            {"action":"call the dentist","date":"2026-06-02","time":null},
            {"action":"pay rent","date":"2026-07-01","time":null,"recurrence":"MONTHLY;BYMONTHDAY=1"}
        ]}"#;
        let v = parse_reminder_intents(raw).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].recurrence.as_deref(), Some("MONTHLY;BYMONTHDAY=1"));
    }

    #[test]
    fn tolerates_markdown_fence() {
        let raw = "```json\n{\"intents\":[{\"action\":\"x\",\"date\":\"2026-06-02\"}]}\n```";
        let v = parse_reminder_intents(raw).unwrap();
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn empty_intents_ok() {
        let v = parse_reminder_intents(r#"{"intents":[]}"#).unwrap();
        assert!(v.is_empty());
    }

    #[test]
    fn no_date_flagged_by_has_date() {
        let raw = r#"{"intents":[{"action":"vague","date":null}]}"#;
        let v = parse_reminder_intents(raw).unwrap();
        assert_eq!(v.len(), 1);
        assert!(!v[0].has_date());
    }

    #[test]
    fn invalid_json_errors() {
        assert!(parse_reminder_intents("not json at all").is_err());
    }
}
