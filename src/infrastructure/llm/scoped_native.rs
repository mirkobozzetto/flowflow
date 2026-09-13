//! Schema-2 model surface. Native tools are always wrapped; external tools come from validated owner sessions.
use super::*;
use crate::application::agent_run::ScopedAgentRun;
use crate::application::tools::scoped_native::mount_native_tools;

impl LlmClient {
    pub async fn run_scoped(
        self: &Arc<Self>,
        model: &str,
        preamble: &str,
        message: &str,
        run: Arc<ScopedAgentRun>,
        allowed: &[String],
        web_key: Option<String>,
        temperature: f64,
        mcp: Vec<(Vec<rmcp::model::Tool>, rmcp::service::ServerSink)>,
    ) -> Result<String, LlmError> {
        if run.aborted() {
            return Err(LlmError::Completion("run actions have ended".into()));
        }
        let native_names = run.native_tool_names();
        let native_allowed: Vec<_> = allowed
            .iter()
            .filter(|name| native_names.contains(&name.as_str()))
            .cloned()
            .collect();
        let tools = mount_native_tools(
            self.clone(),
            run.clone(),
            &native_allowed,
            web_key,
        )
        .map_err(LlmError::Completion)?;
        let hook = run
            .external_hook()
            .scoped_to(allowed.to_vec(), "native".into());
        macro_rules! prompt {
            ($builder:expr) => {{
                let mut builder = $builder
                    .preamble(preamble)
                    .temperature(temperature)
                    .tools(tools);
                for (tools, peer) in mcp {
                    builder = builder.rmcp_tools(tools, peer);
                }
                builder.build()
            }
            .prompt(message)
            .max_turns(4)
            .with_hook(hook)
            .await
            .map_err(|error| LlmError::Completion(error.to_string()))};
        }
        match self.provider {
            Provider::OpenAi => {
                let client = self.openai.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(
                        "OpenAI client not configured".into(),
                    )
                })?;
                prompt!(client.agent(model))
            }
            Provider::Anthropic => {
                let client = self.anthropic.as_ref().ok_or_else(|| {
                    LlmError::NotConfigured(
                        "Anthropic client not configured".into(),
                    )
                })?;
                prompt!(client
                    .agent(ANTHROPIC_CHAT_MODEL)
                    .max_tokens(ANTHROPIC_MAX_TOKENS))
            }
            Provider::ChatGpt => {
                let client = self.chatgpt_client().await?;
                prompt!(client.agent(CHATGPT_CHAT_MODEL).additional_params(
                    serde_json::json!({
                        "reasoning":{"effort":CHATGPT_REASONING_EFFORT}
                    })
                ))
            }
        }
    }
}
