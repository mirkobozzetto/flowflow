use crate::application::constants::{
    ANTHROPIC_CHAT_MODEL, ANTHROPIC_MAX_TOKENS, CHAT_MODEL, EMBEDDING_DIMS,
    EMBEDDING_MODEL,
};
use crate::application::error::LlmError;
use crate::application::tools::{
    ContractHook, CreateNote, ScheduleReminder, SearchNotes, SearchWeb,
    SummarizeFolder,
};
use rig::client::{CompletionClient, EmbeddingsClient};
use rig::completion::Prompt;
use rig::embeddings::EmbeddingModel;
use rig::providers::{anthropic, openai};
use std::str::FromStr;
use std::sync::Arc;

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

/// Whether an agent run gets the local notes tools, and what those tools may see.
///
/// This replaced a bare `bool`. The boolean said "mount the tools" and said nothing
/// about scope, so `search_notes` searched the whole corpus even inside a folder or
/// thread chat - the one place the user had explicitly narrowed. Making the scope
/// part of the same value means a caller cannot mount the tools without deciding it.
#[derive(Clone, Default)]
pub enum NotesTools {
    /// No local notes tools on this run.
    #[default]
    None,
    /// Mounted over every note.
    Global,
    /// Mounted over these notes only (folder, thread, or `@mention`).
    Scoped(Arc<[String]>),
}

impl NotesTools {
    /// `None` = do not mount. `Some(scope)` = mount, `scope` being the id filter
    /// handed to the search tool.
    pub fn scope(&self) -> Option<Option<Arc<[String]>>> {
        match self {
            NotesTools::None => None,
            NotesTools::Global => Some(None),
            NotesTools::Scoped(ids) => Some(Some(ids.clone())),
        }
    }

    /// Mount over `ids` when the chat is scoped, over everything when it is not.
    pub fn from_allowed(ids: Option<&[String]>) -> Self {
        match ids {
            Some(ids) => NotesTools::Scoped(Arc::from(ids)),
            None => NotesTools::Global,
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
        self.chat_with_model(CHAT_MODEL, system, user_message).await
    }

    // The OpenAI model applies only on the OpenAI path; under Anthropic the configured Anthropic
    // model stands (an OpenAI id has no meaning there - cross-provider coherence is a publish-time concern).
    pub(crate) async fn chat_with_model(
        &self,
        openai_model: &str,
        system: &str,
        user_message: &str,
    ) -> Result<String, LlmError> {
        match self.provider {
            Provider::OpenAi => {
                let agent = self
                    .openai
                    .agent(openai_model)
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

    /// Model id the chat/agent path actually uses under the configured provider.
    pub fn chat_model_name(&self) -> &'static str {
        match self.provider {
            Provider::OpenAi => CHAT_MODEL,
            Provider::Anthropic => ANTHROPIC_CHAT_MODEL,
        }
    }

    /// The single agent primitive: one prompt run over an explicit tool surface.
    /// `notes_tools` mounts the three local notes tools; `mcp` mounts a connector tool
    /// subset over its server sink - the caller keeps the registry alive across the await
    /// so the sink stays valid. The `hook` always applies: enforcing when the caller
    /// attached a contract, observe-only otherwise. Nothing else in the app mounts tools.
    ///
    /// The note ids inside `NotesTools` are carried, never interpreted: resolving a
    /// folder or a thread into ids stays in `application`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_agent(
        self: &Arc<Self>,
        openai_model: &str,
        preamble: &str,
        user_message: &str,
        notes_tools: NotesTools,
        web_key: Option<String>,
        mcp: Vec<(Vec<rmcp::model::Tool>, rmcp::service::ServerSink)>,
        hook: ContractHook,
        temperature: f64,
        max_turns: usize,
    ) -> Result<String, LlmError> {
        // rig's AgentBuilder is a typestate: the first `.tool()`/`.rmcp_tools()` changes its
        // type, so the surface combinations are spelled out as match arms instead of
        // conditional reassignment.
        macro_rules! run {
            ($b:expr) => {
                $b.build()
                    .prompt(user_message)
                    .max_turns(max_turns)
                    .with_hook(hook)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()))
            };
        }
        // One `rmcp_tools` call PER connector: each tool clones the sink it was mounted with,
        // so a call routes to its own server. Mounting the merged list against a single sink
        // would send every call to whichever server happened to be first.
        macro_rules! mount {
            ($b:expr, $mcp:expr) => {{
                let mut mounts = $mcp.into_iter();
                match mounts.next() {
                    Some((tools, peer)) => {
                        let mut b = $b.rmcp_tools(tools, peer);
                        for (tools, peer) in mounts {
                            b = b.rmcp_tools(tools, peer);
                        }
                        run!(b)
                    }
                    None => run!($b),
                }
            }};
        }
        // Cloned before the hook is moved into the run: `schedule_reminder` renders as a
        // card, so it needs the same channel the hook reports status on.
        let tool_events = hook.events();
        match self.provider {
            Provider::OpenAi => {
                let base = self
                    .openai
                    .agent(openai_model)
                    .preamble(preamble)
                    .temperature(temperature);
                match (notes_tools.scope(), web_key) {
                    (Some(scope), web) => {
                        let b = base
                            .tool(SearchNotes::new(self.clone(), scope))
                            .tool(CreateNote::new())
                            .tool(SummarizeFolder::new(self.clone()))
                            .tool(ScheduleReminder::new(
                                self.clone(),
                                tool_events.clone(),
                            ));
                        let b = match web {
                            Some(key) => b.tool(SearchWeb::new(key)),
                            None => b,
                        };
                        mount!(b, mcp)
                    }
                    (None, Some(key)) => {
                        mount!(base.tool(SearchWeb::new(key)), mcp)
                    }
                    (None, None) => mount!(base, mcp),
                }
            }
            Provider::Anthropic => {
                let client = self.anthropic.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(
                        "Anthropic client not initialized".into(),
                    )
                })?;
                let base = client
                    .agent(ANTHROPIC_CHAT_MODEL)
                    .preamble(preamble)
                    .temperature(temperature)
                    .max_tokens(ANTHROPIC_MAX_TOKENS);
                match (notes_tools.scope(), web_key) {
                    (Some(scope), web) => {
                        let b = base
                            .tool(SearchNotes::new(self.clone(), scope))
                            .tool(CreateNote::new())
                            .tool(SummarizeFolder::new(self.clone()))
                            .tool(ScheduleReminder::new(
                                self.clone(),
                                tool_events.clone(),
                            ));
                        let b = match web {
                            Some(key) => b.tool(SearchWeb::new(key)),
                            None => b,
                        };
                        mount!(b, mcp)
                    }
                    (None, Some(key)) => {
                        mount!(base.tool(SearchWeb::new(key)), mcp)
                    }
                    (None, None) => mount!(base, mcp),
                }
            }
        }
    }
}

/// The OpenAI chat model an agent runs on: its manifest `model` when set and current, else the
/// default. The retired `gpt-4o` family maps to the default so an in-flight signed manifest that
/// still pins it runs on the current model without forcing a re-sign.
pub fn resolve_chat_model(manifest_model: &str) -> &str {
    match manifest_model.trim() {
        "" | "gpt-4o" | "gpt-4o-mini" => CHAT_MODEL,
        m => m,
    }
}
