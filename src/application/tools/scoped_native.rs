//! Native tools mounted by scoped assistants must use this wrapper, never the
//! raw implementation. Legacy mounts remain unchanged.

use crate::application::agent_run::ScopedAgentRun;
use crate::application::approvals::APPROVAL_TIMEOUT;
use crate::application::tools::{
    CreateNote, ScheduleReminder, SearchNotes, SearchWeb, SummarizeFolder,
    ToolFailure,
};
use crate::infrastructure::llm::LlmClient;
use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolDyn};
use serde_json::Value;
use std::sync::Arc;

pub struct ScopedNativeTool<T> {
    inner: T,
    run: Arc<ScopedAgentRun>,
}
impl<T: Tool> ScopedNativeTool<T> {
    pub fn new(inner: T, run: Arc<ScopedAgentRun>) -> Self {
        Self { inner, run }
    }
}
impl<T: Tool> Tool for ScopedNativeTool<T> {
    const NAME: &'static str = T::NAME;
    type Args = Value;
    type Output = T::Output;
    type Error = ToolFailure;
    async fn definition(&self, prompt: String) -> ToolDefinition {
        self.inner.definition(prompt).await
    }
    async fn call(&self, args: Value) -> Result<T::Output, ToolFailure> {
        self.run
            .execute_native(&self.inner, args, APPROVAL_TIMEOUT)
            .await
    }
}

/// The chain's allowlist narrows the already-admitted native plan. Every mounted
/// object is wrapped, including reads so they consume the shared run budget.
pub fn mount_native_tools(
    llm: Arc<LlmClient>,
    run: Arc<ScopedAgentRun>,
    allowed: &[String],
    web_key: Option<String>,
) -> Result<Vec<Box<dyn ToolDyn>>, String> {
    let admitted = run.native_tool_names();
    let mut tools: Vec<Box<dyn ToolDyn>> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for name in allowed {
        if !seen.insert(name) || !admitted.contains(&name.as_str()) {
            return Err(format!(
                "native tool `{name}` is duplicated or not admitted"
            ));
        }
        let tool: Box<dyn ToolDyn> = match name.as_str() {
            "search_notes" => Box::new(ScopedNativeTool::new(
                SearchNotes::new(llm.clone(), None),
                run.clone(),
            )),
            "create_note" => {
                Box::new(ScopedNativeTool::new(CreateNote::new(), run.clone()))
            }
            "summarize_folder" => Box::new(ScopedNativeTool::new(
                SummarizeFolder::new(llm.clone()),
                run.clone(),
            )),
            "schedule_reminder" => Box::new(ScopedNativeTool::new(
                ScheduleReminder::new(
                    llm.clone(),
                    run.external_hook().events(),
                ),
                run.clone(),
            )),
            "search_web" => {
                let key = web_key
                    .as_ref()
                    .filter(|key| !key.trim().is_empty())
                    .ok_or(
                        "declared native web search has no configured key",
                    )?;
                Box::new(ScopedNativeTool::new(
                    SearchWeb::new(key.clone()),
                    run.clone(),
                ))
            }
            _ => return Err(format!("unsupported native tool `{name}`")),
        };
        tools.push(tool);
    }
    Ok(tools)
}
