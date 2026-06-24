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
    // Ground-truth log of what each tool call actually did (proposed args, gate verdict, raw result), so the
    // chain debug trace shows reality instead of the model's narration. Drained per state by the runtime.
    events: Mutex<Vec<String>>,
}

// Single-line, length-capped view of a JSON-ish blob for the debug log.
fn excerpt(s: &str, max: usize) -> String {
    let one_line: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() > max {
        let head: String = one_line.chars().take(max).collect();
        format!("{head}...")
    } else {
        one_line
    }
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
    // The Layer 2 chain-state filter: when set, only these tools may be proposed in the active state. None =
    // no chain scoping (the single-module path). Independent of the Layer 1 gate; both apply.
    state_allowed: Option<Vec<String>>,
    state_name: Option<String>,
}

impl ContractHook {
    // Observe-only: emits status events, gates nothing. Used until a pinned manifest is available.
    pub fn new(tx: mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self {
            tx,
            contract: None,
            state_allowed: None,
            state_name: None,
        }
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
                events: Mutex::new(Vec::new()),
            })),
            state_allowed: None,
            state_name: None,
        }
    }

    // Share this contract's run accounting with a hook scoped to one chain state's allowed tools. The chain
    // runtime builds one base contract, then a scoped hook per state, so budgets and read_before_write carry
    // across states while each state narrows the visible surface.
    pub fn scoped_to(&self, allowed: Vec<String>, state: String) -> Self {
        Self {
            tx: self.tx.clone(),
            contract: self.contract.clone(),
            state_allowed: Some(allowed),
            state_name: Some(state),
        }
    }

    // Count a chain state as one run step (the gate reads `steps` against `limits.max_steps`). No-op without
    // a contract.
    pub fn bump_step(&self) {
        if let Some(c) = &self.contract {
            c.run.lock().expect("governance run state poisoned").steps += 1;
        }
    }

    // Publish elapsed wall-clock so the gate's `limits.max_run_seconds` ceiling has live data. The chain
    // runtime owns the clock; without it the time bound would never fire. No-op without a contract.
    pub fn set_elapsed(&self, seconds: u64) {
        if let Some(c) = &self.contract {
            c.run
                .lock()
                .expect("governance run state poisoned")
                .elapsed_seconds = seconds;
        }
    }

    // Whether the bound resource has been read in this run, for the FSM-level read_before_write guard.
    pub fn read_bound(&self) -> bool {
        self.contract
            .as_ref()
            .map(|c| {
                c.run
                    .lock()
                    .expect("governance run state poisoned")
                    .read_bound
            })
            .unwrap_or(false)
    }

    // Take this run's accumulated tool-call log (proposed args, gate verdict, raw result). The chain runtime
    // drains it after each state to attach ground truth to that state's trace. Empty without a contract.
    pub fn drain_events(&self) -> Vec<String> {
        self.contract
            .as_ref()
            .map(|c| {
                std::mem::take(&mut *c.events.lock().expect("events poisoned"))
            })
            .unwrap_or_default()
    }

    fn record(&self, line: String) {
        if let Some(c) = &self.contract {
            c.events.lock().expect("events poisoned").push(line);
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

        // Layer 2: a tool the active chain state does not list is refused before the Layer 1 gate runs, so
        // the model self-corrects toward the state's surface.
        if let Some(allowed) = &self.state_allowed {
            if !allowed.iter().any(|t| t == tool_name) {
                self.record(format!(
                    "blocked {tool_name}: not allowed in this chain state"
                ));
                return ToolCallHookAction::Skip {
                    reason: format!(
                        "tool `{tool_name}` is not allowed in chain state `{}` (allowed: {})",
                        self.state_name.as_deref().unwrap_or("?"),
                        allowed.join(", ")
                    ),
                };
            }
        }

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
            Decision::Allow => {
                self.record(format!(
                    "called {tool_name} {} -> allowed",
                    excerpt(args, 160)
                ));
                ToolCallHookAction::Continue
            }
            Decision::Deny(reason) => {
                // `reason` already names the tool, so the log line does not repeat it.
                self.record(format!("blocked: {reason}"));
                ToolCallHookAction::Skip {
                    reason: reason.to_string(),
                }
            }
        }
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
        result: &str,
    ) -> HookAction {
        let _ = self.tx.send(ToolEvent::Finished(tool_name.to_string()));
        self.record(format!("result {tool_name}: {}", excerpt(result, 400)));
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
