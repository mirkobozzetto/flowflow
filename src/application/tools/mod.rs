pub mod create;
pub mod search;
pub mod summarize;

pub use create::{CreateNote, CreateNoteArgs, CreateNoteResult};
pub use search::{SearchNotes, SearchNotesArgs, SearchNotesHit};
pub use summarize::{
    SummarizeFolder, SummarizeFolderArgs, SummarizeFolderResult,
};

use crate::domain::governance::{
    gate, ConnectorManifest, Decision, Governance, ProposedCall, RunState,
};
use crate::infrastructure::llm::LlmClient;
use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum ToolEvent {
    Started(String),
    Finished(String),
}

// The agent's pinned governance + the connector manifest it was validated against, plus the live run
// accounting. The hook borrows `&self`, so the run state sits behind a Mutex (the gate is sync and never
// held across an await). Supplied by the manifest pipeline (M1.15/M1.5); until then the hook runs without
// one and only observes.
struct Contract {
    gov: Governance,
    conn: ConnectorManifest,
    run: Mutex<RunState>,
}

// rig `PromptHook` that enforces the agent behavior contract before each tool call (RFC 0010 M1.12, spec
// 07/09): the device-side seam. It replaces the old observe-only `ToolStatusHook` - it still emits
// `ToolEvent`s for the chat status UI, and, when a `Contract` is attached, applies the governance gate and
// returns `Skip{reason}` on a violation so the model self-corrects. With no contract it is a pure observer,
// which is the current state until an agent manifest is installed and pinned on device.
#[derive(Clone)]
pub struct ContractHook {
    tx: mpsc::UnboundedSender<ToolEvent>,
    contract: Option<Arc<Contract>>,
}

impl ContractHook {
    // Observe-only: emits status events, gates nothing. Used until a pinned manifest is available.
    pub fn new(tx: mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self { tx, contract: None }
    }

    // Enforcing: gate every tool call against `gov` (resolved over `conn`) with fresh per-run accounting.
    pub fn with_contract(
        tx: mpsc::UnboundedSender<ToolEvent>,
        gov: Governance,
        conn: ConnectorManifest,
    ) -> Self {
        Self {
            tx,
            contract: Some(Arc::new(Contract {
                gov,
                conn,
                run: Mutex::new(RunState::default()),
            })),
        }
    }
}

impl<M: CompletionModel> PromptHook<M> for ContractHook {
    async fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        args: &str,
    ) -> ToolCallHookAction {
        let _ = self.tx.send(ToolEvent::Started(tool_name.to_string()));

        let Some(contract) = &self.contract else {
            return ToolCallHookAction::Continue;
        };

        // The model passes tool args as a JSON string; a non-object payload pins nothing (bound checks
        // treat it as a miss). Column extraction is connector-specific and deferred, so no touched_columns.
        let parsed =
            serde_json::from_str(args).unwrap_or(serde_json::Value::Null);
        let call = ProposedCall::new(tool_name, parsed);
        let decision = {
            let mut run =
                contract.run.lock().expect("governance run state poisoned");
            gate(&contract.gov, &contract.conn, &call, &mut run)
        };
        match decision {
            Decision::Allow => ToolCallHookAction::Continue,
            Decision::Deny(reason) => ToolCallHookAction::Skip {
                reason: reason.to_string(),
            },
        }
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
) -> Result<String, crate::application::error::LlmError> {
    llm.prompt_with_agent(preamble, user_message, status_tx)
        .await
}
