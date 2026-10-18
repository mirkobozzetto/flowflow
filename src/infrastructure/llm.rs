#[cfg(debug_assertions)]
pub mod claude_code;
mod reading;
mod scoped_native;

use crate::application::constants::{
    ANTHROPIC_CHAT_MODEL, ANTHROPIC_MAX_TOKENS, CHATGPT_CHAT_MODEL,
    CHATGPT_REASONING_EFFORT, CHAT_MODEL, EMBEDDING_DIMS, EMBEDDING_MODEL,
};
use crate::application::error::LlmError;
use crate::application::tools::{
    ContractHook, CreateNote, ScheduleReminder, SearchNotes, SearchWeb,
    SummarizeFolder,
};
use rig::client::{CompletionClient, EmbeddingsClient};
use rig::completion::Prompt;
use rig::embeddings::EmbeddingModel;
use rig::providers::{anthropic, chatgpt, openai};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Provider {
    #[default]
    OpenAi,
    Anthropic,
    ChatGpt,
    #[cfg(debug_assertions)]
    ClaudeSubscription,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::OpenAi => "openai",
            Provider::Anthropic => "anthropic",
            Provider::ChatGpt => "chatgpt",
            #[cfg(debug_assertions)]
            Provider::ClaudeSubscription => "claude_subscription",
        }
    }

    /// A subscription answers the chat without an OpenAI key; embeddings
    /// still need one.
    pub fn is_subscription(self) -> bool {
        match self {
            Provider::ChatGpt => true,
            #[cfg(debug_assertions)]
            Provider::ClaudeSubscription => true,
            Provider::OpenAi | Provider::Anthropic => false,
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
            "chatgpt" | "chat_gpt" | "subscription" => Ok(Provider::ChatGpt),
            // A release build reads it as unknown: back to the OpenAI key.
            #[cfg(debug_assertions)]
            "claude_subscription" => Ok(Provider::ClaudeSubscription),
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
    openai: Option<openai::Client>,
    provider: Provider,
    anthropic: Option<anthropic::Client>,
    lang: String,
    // The model and reasoning level a chat answer runs on (chat_model.rs);
    // None keeps each provider's default.
    chat_pick: Option<(String, String)>,
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
        let provider = db
            .get_setting("llm_provider")
            .and_then(|v| Provider::from_str(&v).ok())
            .unwrap_or_default();
        Self::for_provider(db, provider)
    }

    pub fn for_provider(
        db: &crate::infrastructure::persistence::Database,
        provider: Provider,
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
            .or_else(|| crate::baked_key!("OPENAI_API_KEY"))
            .filter(|key| !key.is_empty() && key != "your_key_here");
        if !provider.is_subscription() && openai_key.is_none() {
            return Err(LlmError::NotConfigured(crate::application::i18n::t(
                &lang,
                "llm-error-openai-key",
            )));
        }
        let openai = openai_key
            .map(|key| openai::Client::new(&key))
            .transpose()
            .map_err(|e| LlmError::NotConfigured(e.to_string()))?;

        let anthropic = if provider == Provider::Anthropic {
            let key = db
                .get_setting("anthropic_api_key")
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .or_else(|| crate::baked_key!("ANTHROPIC_API_KEY"))
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
            lang,
            chat_pick: None,
        })
    }

    /// The model and reasoning level the agent runs on (`run_agent`).
    pub fn with_chat_model(mut self, model: &str, level: &str) -> Self {
        self.chat_pick = Some((model.to_owned(), level.to_owned()));
        self
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        let client = self.openai.as_ref().ok_or_else(|| {
            LlmError::NotConfigured(crate::application::i18n::t(
                &self.lang,
                "llm-error-openai-key",
            ))
        })?;
        let model =
            client.embedding_model_with_ndims(EMBEDDING_MODEL, EMBEDDING_DIMS);
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
                let client = self.openai.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(crate::application::i18n::t(
                        &self.lang,
                        "llm-error-openai-key",
                    ))
                })?;
                let agent = client
                    .agent(openai_model)
                    .preamble(system)
                    .temperature(0.3)
                    .build();
                agent
                    .prompt(user_message)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()))
            }
            // Claude 5.5 models reject any temperature: none is sent to Anthropic.
            Provider::Anthropic => {
                let client = self.anthropic.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(
                        "Anthropic client not initialized".into(),
                    )
                })?;
                let agent = client
                    .agent(ANTHROPIC_CHAT_MODEL)
                    .preamble(system)
                    .max_tokens(ANTHROPIC_MAX_TOKENS)
                    .build();
                agent
                    .prompt(user_message)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()))
            }
            Provider::ChatGpt => {
                let client = self.chatgpt_client().await?;
                let agent = client
                    .agent(CHATGPT_CHAT_MODEL)
                    .preamble(system)
                    .temperature(0.3)
                    .additional_params(serde_json::json!({
                        "reasoning": { "effort": CHATGPT_REASONING_EFFORT }
                    }))
                    .build();
                agent
                    .prompt(user_message)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()))
            }
            #[cfg(debug_assertions)]
            Provider::ClaudeSubscription => {
                let client = self.claude_subscription_client().await?;
                let agent = client
                    .agent(ANTHROPIC_CHAT_MODEL)
                    .preamble(system)
                    .max_tokens(ANTHROPIC_MAX_TOKENS)
                    .build();
                let reply = agent
                    .prompt(user_message)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()));
                self.claude_subscription_reply(reply)
            }
        }
    }

    /// Model actually executed when `openai_model` is requested: the OpenAI
    /// path honors it; Anthropic and ChatGPT substitute their own model.
    pub fn effective_model(&self, openai_model: &'static str) -> &'static str {
        match self.provider {
            Provider::OpenAi => openai_model,
            Provider::Anthropic => ANTHROPIC_CHAT_MODEL,
            Provider::ChatGpt => CHATGPT_CHAT_MODEL,
            #[cfg(debug_assertions)]
            Provider::ClaudeSubscription => ANTHROPIC_CHAT_MODEL,
        }
    }

    /// Model id the chat/agent path actually uses under the configured provider.
    pub fn chat_model_name(&self) -> &str {
        match &self.chat_pick {
            Some((model, _)) => model,
            None => self.effective_model(CHAT_MODEL),
        }
    }

    async fn chatgpt_client(&self) -> Result<chatgpt::Client, LlmError> {
        let (access_token, account_id) =
            crate::infrastructure::chatgpt_auth::access_context()
                .await
                .map_err(|_| {
                    LlmError::NotConfigured(crate::application::i18n::t(
                        &self.lang,
                        "llm-error-chatgpt-auth",
                    ))
                })?;
        chatgpt::Client::builder()
            .api_key(chatgpt::ChatGPTAuth::AccessToken {
                access_token,
                account_id,
            })
            .build()
            .map_err(|e| LlmError::NotConfigured(e.to_string()))
    }

    #[cfg(debug_assertions)]
    pub(crate) async fn claude_subscription_client(
        &self,
    ) -> Result<claude_code::ClaudeCodeClient, LlmError> {
        use crate::infrastructure::claude_auth::{self, AccessError};
        let token = claude_auth::access_token().await.map_err(|e| match e {
            AccessError::NotConnected => {
                LlmError::NotConfigured(crate::application::i18n::t(
                    &self.lang,
                    "llm-error-claude-auth",
                ))
            }
            AccessError::Failed(error) => LlmError::Completion(error),
        })?;
        claude_code::client(&token)
            .map_err(|e| LlmError::NotConfigured(e.to_string()))
    }

    /// Every subscription answer leaves a log line. Anthropic bills a
    /// third-party app's call as extra usage when it does not fit the
    /// subscription: said plainly, never passed off as an answer.
    #[cfg(debug_assertions)]
    pub(crate) fn claude_subscription_reply(
        &self,
        reply: Result<String, LlmError>,
    ) -> Result<String, LlmError> {
        let error = match reply {
            Ok(answer) => {
                eprintln!("[claude] answered via subscription ({ANTHROPIC_CHAT_MODEL})");
                return Ok(answer);
            }
            Err(LlmError::Completion(error)) => error,
            Err(other) => return Err(other),
        };
        eprintln!("[claude] request failed: {error}");
        if error.contains("429 Too Many Requests")
            || error.to_ascii_lowercase().contains("extra usage")
        {
            return Err(LlmError::NotConfigured(crate::application::i18n::t(
                &self.lang,
                "llm-error-claude-extra-usage",
            )));
        }
        Err(LlmError::Completion(error))
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
        let llm = self.clone();
        macro_rules! surface {
            ($base:expr) => {{
                let base = $base;
                match (notes_tools.scope(), web_key) {
                    (Some(scope), web) => {
                        let b = base
                            .tool(SearchNotes::new(llm.clone(), scope))
                            .tool(CreateNote::new())
                            .tool(SummarizeFolder::new(llm.clone()))
                            .tool(ScheduleReminder::new(
                                llm.clone(),
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
            }};
        }
        match self.provider {
            Provider::OpenAi => {
                let client = self.openai.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(crate::application::i18n::t(
                        &self.lang,
                        "llm-error-openai-key",
                    ))
                })?;
                // A picked model reasons at its level; the GPT-6 models
                // take no temperature.
                let base = match &self.chat_pick {
                    Some((model, level)) => client
                        .agent(model)
                        .preamble(preamble)
                        .additional_params(serde_json::json!({
                            "reasoning": { "effort": level }
                        })),
                    None => client
                        .agent(openai_model)
                        .preamble(preamble)
                        .temperature(temperature),
                };
                surface!(base)
            }
            Provider::Anthropic => {
                let client = self.anthropic.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(
                        "Anthropic client not initialized".into(),
                    )
                })?;
                surface!(self.claude_agent(client, preamble))
            }
            Provider::ChatGpt => {
                let client = self.chatgpt_client().await?;
                let base = match &self.chat_pick {
                    Some((model, level)) => client
                        .agent(model)
                        .preamble(preamble)
                        .additional_params(serde_json::json!({
                            "reasoning": { "effort": level }
                        })),
                    None => client
                        .agent(CHATGPT_CHAT_MODEL)
                        .preamble(preamble)
                        .temperature(temperature)
                        .additional_params(serde_json::json!({
                            "reasoning": { "effort": CHATGPT_REASONING_EFFORT }
                        })),
                };
                surface!(base)
            }
            #[cfg(debug_assertions)]
            Provider::ClaudeSubscription => {
                let client = self.claude_subscription_client().await?;
                let reply = surface!(self.claude_agent(&client, preamble));
                self.claude_subscription_reply(reply)
            }
        }
    }
}

impl LlmClient {
    /// A Claude agent on the picked model and level (Claude 5.5 sets the
    /// level through `output_config.effort`), else on the default model.
    fn claude_agent<H>(
        &self,
        client: &anthropic::Client<H>,
        preamble: &str,
    ) -> rig::agent::AgentBuilder<anthropic::completion::CompletionModel<H>>
    where
        H: rig::http_client::HttpClientExt
            + Clone
            + Default
            + std::fmt::Debug
            + rig::wasm_compat::WasmCompatSend
            + 'static,
    {
        let (model, level) = match &self.chat_pick {
            Some((model, level)) => (model.as_str(), Some(level)),
            None => (ANTHROPIC_CHAT_MODEL, None),
        };
        let base = client
            .agent(model)
            .preamble(preamble)
            .max_tokens(ANTHROPIC_MAX_TOKENS);
        match level {
            Some(level) => base.additional_params(serde_json::json!({
                "output_config": { "effort": level }
            })),
            None => base,
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
