pub mod create;
pub mod search;
pub mod summarize;

pub use create::{CreateNote, CreateNoteArgs, CreateNoteResult};
pub use search::{SearchNotes, SearchNotesArgs, SearchNotesHit};
pub use summarize::{
    SummarizeFolder, SummarizeFolderArgs, SummarizeFolderResult,
};

use crate::services::llm::LlmClient;
use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum ToolEvent {
    Started(String),
    Finished(String),
}

#[derive(Clone)]
pub struct ToolStatusHook {
    tx: mpsc::UnboundedSender<ToolEvent>,
}

impl ToolStatusHook {
    pub fn new(tx: mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self { tx }
    }
}

impl<M: CompletionModel> PromptHook<M> for ToolStatusHook {
    async fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
    ) -> ToolCallHookAction {
        let _ = self.tx.send(ToolEvent::Started(tool_name.to_string()));
        ToolCallHookAction::Continue
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
        _result: &str,
    ) -> HookAction {
        let _ = self.tx.send(ToolEvent::Finished(tool_name.to_string()));
        HookAction::cont()
    }
}

#[derive(Debug)]
pub struct ToolFailure(pub String);

impl std::fmt::Display for ToolFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolFailure {}

pub async fn prompt_agent_with_tools(
    llm: Arc<LlmClient>,
    preamble: &str,
    user_message: &str,
    status_tx: Option<mpsc::UnboundedSender<ToolEvent>>,
) -> Result<String, crate::services::error::LlmError> {
    llm.prompt_with_agent(preamble, user_message, status_tx)
        .await
}
