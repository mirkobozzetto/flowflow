pub mod create;
pub mod search;
pub mod summarize;

pub use create::{CreateNote, CreateNoteArgs, CreateNoteResult};
pub use search::{SearchNotes, SearchNotesArgs, SearchNotesHit};
pub use summarize::{
    SummarizeFolder, SummarizeFolderArgs, SummarizeFolderResult,
};

use crate::application::approvals::{
    self, Outcome, ProposalView, APPROVAL_TIMEOUT,
};
use crate::domain::governance::{
    call_fingerprint, gate, is_row_tool, row_tool_touched_columns,
    validate_row_batch, ConnectorManifest, Decision, DenyReason, Governance,
    ProposedCall, RunState,
};
use crate::infrastructure::llm::LlmClient;
use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Clone, Debug, PartialEq)]
pub enum ProposalStatus {
    Approved,
    Edited,
    Rejected,
    Expired,
}

#[derive(Clone, Debug)]
pub enum ToolEvent {
    Started(String),
    Finished(String),
    /// A write is suspended pending the user's decision: render the card.
    Proposal(ProposalView),
    /// The suspended write was decided (or expired): freeze the card.
    ProposalResolved {
        id: String,
        status: ProposalStatus,
    },
    /// The user's edit failed re-validation: show the reason, the card stays pending.
    EditRejected {
        id: String,
        reason: String,
    },
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
    // Seconds spent suspended on approval cards: the chain runtime subtracts this from wall
    // clock, so `max_run_seconds` bounds compute time, never user think-time.
    held_secs: AtomicU64,
    // A rejected/expired card ends the run's remaining write intents: the chain runtime jumps
    // to the terminal answer instead of `on_done`.
    abort: AtomicBool,
    // The run's snapshot of the armed sheet schema ({spreadsheet_id: {tab: {headers, captured_at}}}).
    // The hook validates row writes against it and may refresh a (sheet, tab) entry through the MCP
    // peer ONCE per run - an edit -> re-gate never re-syncs. The hook holds no Database, so refreshed
    // entries persist at run end via schema_snapshot() -> store_schema_map (the chain runtime owns the db).
    schema: Mutex<serde_json::Map<String, serde_json::Value>>,
    resynced: Mutex<BTreeSet<String>>,
    schema_dirty: AtomicBool,
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
    // MCP sink for executing an approved EDITED payload directly; None = edits fall back to
    // reject (no deterministic way to run the user's bytes).
    peer: Option<rmcp::service::ServerSink>,
    // Card deadline; APPROVAL_TIMEOUT in production, shrunk by tests.
    approval_timeout: std::time::Duration,
}

impl ContractHook {
    // Observe-only: emits status events, gates nothing. Used until a pinned manifest is available.
    pub fn new(tx: mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self {
            tx,
            contract: None,
            state_allowed: None,
            state_name: None,
            peer: None,
            approval_timeout: APPROVAL_TIMEOUT,
        }
    }

    // Enforcing: gate every tool call against `gov` (resolved over `conn`) with fresh per-run accounting.
    // `schema` is the armed sheet schema snapshot header-keyed row writes validate against.
    pub fn with_contract(
        tx: mpsc::UnboundedSender<ToolEvent>,
        gov: Governance,
        conn: ConnectorManifest,
        schema: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            tx,
            contract: Some(Arc::new(Contract {
                gov,
                conn,
                run: Mutex::new(RunState::default()),
                events: Mutex::new(Vec::new()),
                held_secs: AtomicU64::new(0),
                abort: AtomicBool::new(false),
                schema: Mutex::new(schema),
                resynced: Mutex::new(BTreeSet::new()),
                schema_dirty: AtomicBool::new(false),
            })),
            state_allowed: None,
            state_name: None,
            peer: None,
            approval_timeout: APPROVAL_TIMEOUT,
        }
    }

    /// The schema map as refreshed by this run's re-syncs, or None when nothing changed.
    /// The chain runtime persists it (the hook never holds the Database).
    pub fn schema_snapshot(
        &self,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let contract = self.contract.as_ref()?;
        if !contract.schema_dirty.load(Ordering::Relaxed) {
            return None;
        }
        Some(contract.schema.lock().expect("schema poisoned").clone())
    }

    // Validate a header-keyed row write against the armed sheet schema. On a schema-shaped refusal
    // (unknown column, no captured schema) the targeted tab's headers are re-read through the MCP
    // peer AT MOST once per run - the sheet may have been legitimately restructured - then the
    // batch re-validates. Still failing = the returned DenyReason (it names the REAL headers, so
    // the model self-corrects). A schema that stays unknown refuses the write rather than writing blind.
    async fn validate_row_write(
        &self,
        contract: &Arc<Contract>,
        call: &ProposedCall,
    ) -> Option<DenyReason> {
        let sid = call.args.get("spreadsheet_id").and_then(|v| v.as_str());
        let sheet = call.args.get("sheet").and_then(|v| v.as_str());
        let (Some(sid), Some(sheet)) = (sid, sheet) else {
            // No addressable target: run the key/batch rules; the gate's bound check (and the
            // executor's own arg validation) owns the missing-target refusal.
            return validate_row_batch(
                &contract.gov,
                &call.tool,
                &call.args,
                None,
            );
        };

        let lookup = |contract: &Contract| -> Option<Vec<String>> {
            contract
                .schema
                .lock()
                .expect("schema poisoned")
                .get(sid)?
                .get(sheet)?
                .get("headers")?
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
        };

        let mut headers = lookup(contract);
        let mut reason = validate_row_batch(
            &contract.gov,
            &call.tool,
            &call.args,
            headers.as_deref(),
        );

        let schema_shaped = headers.is_none()
            || matches!(
                reason,
                Some(DenyReason::SchemaMismatch { .. })
                    | Some(DenyReason::NoHeaderRow { .. })
            );
        let first_resync = schema_shaped
            && contract
                .resynced
                .lock()
                .expect("resync set poisoned")
                .insert(format!("{sid}\u{1f}{sheet}"));
        if let (true, Some(peer)) = (first_resync, &self.peer) {
            match crate::application::connector_module::fetch_tab_headers(
                peer, sid, sheet,
            )
            .await
            {
                Ok(raw) => {
                    match crate::application::connector_module::validate_headers(
                        &raw,
                    ) {
                        Ok(fresh) => {
                            let mut schema = contract
                                .schema
                                .lock()
                                .expect("schema poisoned");
                            let entry = schema
                                .entry(sid.to_string())
                                .or_insert_with(|| serde_json::json!({}));
                            if let Some(tabs) = entry.as_object_mut() {
                                tabs.insert(
                                    sheet.to_string(),
                                    crate::application::connector_module::tab_schema_entry(&fresh),
                                );
                            }
                            contract
                                .schema_dirty
                                .store(true, Ordering::Relaxed);
                            headers = Some(fresh);
                        }
                        // The live header row itself is unusable (duplicate/empty cells): keep the
                        // stored schema, log why - the refusal below carries the actionable message.
                        Err(msg) => self.record(format!(
                            "re-sync refused for {sheet}: {msg}"
                        )),
                    }
                }
                Err(e) => self.record(format!("re-sync failed: {e}")),
            }
            reason = validate_row_batch(
                &contract.gov,
                &call.tool,
                &call.args,
                headers.as_deref(),
            );
        }

        if reason.is_none() && headers.is_none() {
            reason = Some(DenyReason::SchemaUnknown {
                tool: call.tool.clone(),
            });
        }
        reason
    }

    // Attach the MCP server sink so an approved EDITED payload executes deterministically
    // (the hook dials it; rig's Continue can only run the model's original args).
    pub fn with_peer(mut self, peer: rmcp::service::ServerSink) -> Self {
        self.peer = Some(peer);
        self
    }

    /// Seconds spent suspended on approval cards this run.
    pub fn held_seconds(&self) -> u64 {
        self.contract
            .as_ref()
            .map(|c| c.held_secs.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Whether a rejected/expired card asked the run to conclude instead of advancing.
    pub fn aborted(&self) -> bool {
        self.contract
            .as_ref()
            .map(|c| c.abort.load(Ordering::Relaxed))
            .unwrap_or(false)
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
            peer: self.peer.clone(),
            approval_timeout: self.approval_timeout,
        }
    }

    /// Shrink the card deadline (tests only; production keeps APPROVAL_TIMEOUT).
    pub fn with_approval_timeout(
        mut self,
        timeout: std::time::Duration,
    ) -> Self {
        self.approval_timeout = timeout;
        self
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

    // Whether ANY bound resource was read this run, for the coarse FSM-level read_before_write guard. The
    // gate enforces the precise per-resource rule; this only gates entry into a write state.
    pub fn read_any(&self) -> bool {
        self.contract
            .as_ref()
            .map(|c| {
                !c.run
                    .lock()
                    .expect("governance run state poisoned")
                    .read_resources
                    .is_empty()
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

    fn resolve_card(&self, id: uuid::Uuid, status: ProposalStatus) {
        let _ = self.tx.send(ToolEvent::ProposalResolved {
            id: id.to_string(),
            status,
        });
    }

    // Execute the user's corrected payload deterministically against the MCP sink: rig's
    // Continue can only run the model's ORIGINAL args, so the hook dials the edit itself.
    async fn execute_direct(
        &self,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<String, String> {
        let peer = self.peer.as_ref().ok_or("no MCP peer attached")?;
        let mut params =
            rmcp::model::CallToolRequestParams::new(tool.to_string());
        if let Some(obj) = args.as_object() {
            params = params.with_arguments(obj.clone());
        }
        let result = peer.call_tool(params).await.map_err(|e| e.to_string())?;
        Ok(excerpt(
            &serde_json::to_string(&result).unwrap_or_default(),
            400,
        ))
    }

    // The suspended-write flow: card out, await the user, act on the verdict. The gate lock
    // is NEVER held across an await; the registry entry dies with this future (Drop guard).
    async fn approval_flow(
        &self,
        contract: &Arc<Contract>,
        proposal: crate::domain::governance::Proposal,
    ) -> ToolCallHookAction {
        let mut registered = approvals::register(self.approval_timeout);
        let card_id = registered.id();
        let _ = self.tx.send(ToolEvent::Proposal(approvals::view_for(
            card_id,
            &proposal,
            &contract.conn,
        )));

        let hold_start = std::time::Instant::now();
        let action = loop {
            match approvals::await_decision(registered).await {
                Outcome::Approved => {
                    let decision = {
                        let mut run = contract
                            .run
                            .lock()
                            .expect("governance run state poisoned");
                        run.approvals.insert(proposal.fingerprint.clone());
                        let call = ProposedCall::new(
                            proposal.tool.clone(),
                            proposal.args.clone(),
                        );
                        gate(&contract.gov, &contract.conn, &call, &mut run)
                    };
                    match decision {
                        Decision::Allow => {
                            self.record(format!(
                                "approved {} -> executing",
                                proposal.tool
                            ));
                            self.resolve_card(
                                card_id,
                                ProposalStatus::Approved,
                            );
                            break ToolCallHookAction::Continue;
                        }
                        // A budget/limit raced the approval: fail closed, never execute.
                        other => {
                            let reason = match other {
                                Decision::Deny(r) => r.to_string(),
                                _ => "approval re-check did not allow the call"
                                    .into(),
                            };
                            self.record(format!(
                                "approved {} but re-gate refused: {reason}",
                                proposal.tool
                            ));
                            self.resolve_card(
                                card_id,
                                ProposalStatus::Rejected,
                            );
                            break ToolCallHookAction::Skip { reason };
                        }
                    }
                }
                Outcome::Edited(new_args) => {
                    let fp = call_fingerprint(&proposal.tool, &new_args);
                    // An edited row write re-validates against the schema too - but never
                    // re-syncs (this run already spent its one re-sync for that tab).
                    let edited_call = ProposedCall::new(
                        proposal.tool.clone(),
                        new_args.clone(),
                    );
                    let row_refusal = if is_row_tool(&proposal.tool) {
                        self.validate_row_write(contract, &edited_call).await
                    } else {
                        None
                    };
                    let decision = if let Some(reason) = row_refusal {
                        Decision::Deny(reason)
                    } else {
                        let mut run = contract
                            .run
                            .lock()
                            .expect("governance run state poisoned");
                        run.approvals.insert(fp.clone());
                        let call = edited_call.clone().with_columns(
                            row_tool_touched_columns(&edited_call.args),
                        );
                        gate(&contract.gov, &contract.conn, &call, &mut run)
                    };
                    match decision {
                        Decision::Allow => {
                            let outcome = self
                                .execute_direct(&proposal.tool, &new_args)
                                .await;
                            match outcome {
                                Ok(result) => {
                                    self.record(format!(
                                        "edited {} {} -> executed",
                                        proposal.tool,
                                        excerpt(&new_args.to_string(), 160)
                                    ));
                                    self.resolve_card(
                                        card_id,
                                        ProposalStatus::Edited,
                                    );
                                    break ToolCallHookAction::Skip {
                                        reason: format!(
                                            "the user corrected the payload; `{}` was already executed with {} and returned: {result}. Do not call it again for this step.",
                                            proposal.tool, new_args
                                        ),
                                    };
                                }
                                Err(e) => {
                                    self.record(format!(
                                        "edited {} -> execution failed: {e}",
                                        proposal.tool
                                    ));
                                    self.resolve_card(
                                        card_id,
                                        ProposalStatus::Rejected,
                                    );
                                    break ToolCallHookAction::Skip {
                                        reason: format!(
                                            "the user-corrected call failed: {e}; report it, do not retry"
                                        ),
                                    };
                                }
                            }
                        }
                        Decision::Deny(reason) => {
                            // The gate consumes the grant only on Allow: clean it up, tell
                            // the card, and keep waiting on the SAME card identity.
                            contract
                                .run
                                .lock()
                                .expect("governance run state poisoned")
                                .approvals
                                .remove(&fp);
                            let _ = self.tx.send(ToolEvent::EditRejected {
                                id: card_id.to_string(),
                                reason: reason.to_string(),
                            });
                            registered = approvals::register_with_id(
                                card_id,
                                self.approval_timeout,
                            );
                            continue;
                        }
                        Decision::Hold(_) => {
                            // Unreachable: the grant was just inserted. Fail closed.
                            self.resolve_card(
                                card_id,
                                ProposalStatus::Rejected,
                            );
                            break ToolCallHookAction::Skip {
                                reason: "approval re-check held the call again; aborting".into(),
                            };
                        }
                    }
                }
                Outcome::Rejected => {
                    contract.abort.store(true, Ordering::Relaxed);
                    self.record(format!("rejected {}", proposal.tool));
                    self.resolve_card(card_id, ProposalStatus::Rejected);
                    break ToolCallHookAction::Skip {
                        reason: "the user rejected this write; do not retry it"
                            .into(),
                    };
                }
                Outcome::Expired => {
                    contract.abort.store(true, Ordering::Relaxed);
                    self.record(format!(
                        "expired {} (no user decision)",
                        proposal.tool
                    ));
                    self.resolve_card(card_id, ProposalStatus::Expired);
                    break ToolCallHookAction::Skip {
                        reason: "the approval request expired; do not retry"
                            .into(),
                    };
                }
            }
        };
        contract
            .held_secs
            .fetch_add(hold_start.elapsed().as_secs(), Ordering::Relaxed);
        action
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
        // treat it as a miss).
        let parsed =
            serde_json::from_str(args).unwrap_or(serde_json::Value::Null);

        // Header-keyed row writes: validate the batch against the armed sheet schema BEFORE the
        // gate, re-syncing the tab's headers once per run on a schema-shaped refusal. The touched
        // columns become real here, so the gate's column_roles check applies to row writes too.
        let mut call = ProposedCall::new(tool_name, parsed);
        if is_row_tool(tool_name) {
            if let Some(reason) = self.validate_row_write(contract, &call).await
            {
                self.record(format!("blocked: {reason}"));
                return ToolCallHookAction::Skip {
                    reason: reason.to_string(),
                };
            }
            call = call
                .clone()
                .with_columns(row_tool_touched_columns(&call.args));
        }
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
            Decision::Hold(proposal) => {
                self.record(format!(
                    "held {tool_name} {} -> awaiting user approval",
                    excerpt(args, 160)
                ));
                self.approval_flow(contract, proposal).await
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
