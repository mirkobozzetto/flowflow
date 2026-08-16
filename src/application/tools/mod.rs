pub mod create;
pub mod search;
pub mod summarize;
pub mod web;

pub use create::{CreateNote, CreateNoteArgs, CreateNoteResult};
pub use search::{SearchNotes, SearchNotesArgs, SearchNotesHit};
pub use summarize::{
    SummarizeFolder, SummarizeFolderArgs, SummarizeFolderResult,
};
pub use web::{SearchWeb, SearchWebArgs, SearchWebHit};

use crate::application::approvals::{
    self, Outcome, ProposalView, APPROVAL_TIMEOUT,
};
use crate::domain::governance::{
    bound_pins, bound_tabs_for, bound_values, call_fingerprint, gate,
    is_row_tool, row_tool_touched_columns, validate_row_batch,
    ConnectorManifest, Decision, DenyReason, Governance, ProposedCall,
    RunState,
};
use crate::infrastructure::llm::{LlmClient, NotesTools};
use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;
use std::collections::{BTreeMap, BTreeSet};
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

// One connector's governance + the manifest it was validated against, plus the live run
// accounting. The hook borrows `&self`, so the run state sits behind a Mutex (the gate is sync and never
// held across an await). Chains pin one from the installed agent; the chat surface derives one per
// connected connector.
struct Contract {
    gov: Governance,
    conn: ConnectorManifest,
    // Shared by every connector of the run: `max_tool_calls` and the budgets bound the whole
    // run, not each connector separately.
    run: Arc<Mutex<RunState>>,
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

/// A string at a dotted path inside a JSON item ("id", "properties.title").
/// `pub` so the response-filter semantics are pinned by tests without a live peer.
pub fn path_str(item: &serde_json::Value, path: &str) -> Option<String> {
    let mut cur = item;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    cur.as_str().map(String::from)
}

/// Retain only the items at `items_path` that `keep` accepts; the rest of the response passes
/// through, except a `total_count` sibling which is rewritten to the kept length (the original
/// count would leak how many resources are masked). None = the path is missing or not an array
/// (the caller fails closed). `pub` for the same test seam as `path_str`.
pub fn filter_items(
    mut response: serde_json::Value,
    items_path: &str,
    keep: impl Fn(&serde_json::Value) -> bool,
) -> Option<serde_json::Value> {
    let mut cur = &mut response;
    for seg in items_path.split('.') {
        cur = cur.get_mut(seg)?;
    }
    let arr = cur.as_array_mut()?;
    arr.retain(|item| keep(item));
    let kept = arr.len();
    if let Some(obj) = response.as_object_mut() {
        if obj.contains_key("total_count") {
            obj.insert("total_count".into(), kept.into());
        }
    }
    Some(response)
}

// The fail-closed shape: an empty items array, nothing else from the original response.
fn empty_scoped(items_path: &str) -> serde_json::Value {
    let leaf = items_path.split('.').next_back().unwrap_or(items_path);
    serde_json::json!({ leaf: [] })
}

// The filtered result as the LLM will see it, with the fixed scope note attached.
fn with_note(mut filtered: serde_json::Value, note: &str) -> String {
    if let Some(obj) = filtered.as_object_mut() {
        obj.insert("note".into(), note.into());
    }
    filtered.to_string()
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

// rig `PromptHook` that enforces the behavior contract before each tool call: the device-side seam.
// It emits `ToolEvent`s for the chat status UI and, when contracts are attached, applies the
// governance gate and returns `Skip{reason}` on a violation so the model self-corrects. With no
// contract it is a pure observer (notes-only surfaces).
// Local tools pass the gate untouched: they are constructed in-process, never MCP, and reach no
// connector resource. `search_web` is read-only and writes nothing, so it needs no contract.
// The chat surface asserts no connector tool collides with these names at build time.
pub const NOTES_TOOL_NAMES: [&str; 4] = [
    "search_notes",
    "create_note",
    "summarize_folder",
    "search_web",
];

#[derive(Clone)]
pub struct ContractHook {
    tx: mpsc::UnboundedSender<ToolEvent>,
    // (mcp_prefix, contract) pairs. Empty = observe-only (gates nothing). The chain path
    // is one catch-all pair with prefix "" (matches every tool, single-contract semantics
    // unchanged). The chat surface holds one pair per connected connector, resolved per
    // call by longest prefix match; an MCP tool matching NO pair is refused (fail closed).
    contracts: Vec<(String, Arc<Contract>)>,
    // The Layer 2 chain-state filter: when set, only these tools may be proposed in the active state. None =
    // no chain scoping (the single-module path). Independent of the Layer 1 gate; both apply.
    state_allowed: Option<Vec<String>>,
    state_name: Option<String>,
    // MCP sink for executing an approved EDITED payload directly; None = edits fall back to
    // reject (no deterministic way to run the user's bytes).
    peer: Option<rmcp::service::ServerSink>,
    // Which server serves each tool, when several connectors are mounted. Empty otherwise:
    // `peer` is the fallback.
    peers: BTreeMap<String, rmcp::service::ServerSink>,
    // Card deadline; APPROVAL_TIMEOUT in production, shrunk by tests.
    approval_timeout: std::time::Duration,
}

impl ContractHook {
    // Observe-only: emits status events, gates nothing. Used by paths that mount no
    // connector tools (notes-only surfaces).
    pub fn new(tx: mpsc::UnboundedSender<ToolEvent>) -> Self {
        Self {
            tx,
            contracts: Vec::new(),
            state_allowed: None,
            state_name: None,
            peer: None,
            peers: BTreeMap::new(),
            approval_timeout: APPROVAL_TIMEOUT,
        }
    }

    fn make_contract(
        gov: Governance,
        conn: ConnectorManifest,
        schema: serde_json::Map<String, serde_json::Value>,
        run: Arc<Mutex<RunState>>,
    ) -> Arc<Contract> {
        Arc::new(Contract {
            gov,
            conn,
            run,
            events: Mutex::new(Vec::new()),
            held_secs: AtomicU64::new(0),
            abort: AtomicBool::new(false),
            schema: Mutex::new(schema),
            resynced: Mutex::new(BTreeSet::new()),
            schema_dirty: AtomicBool::new(false),
        })
    }

    // Enforcing, single contract: gate every tool call against `gov` (resolved over `conn`)
    // with fresh per-run accounting. The catch-all "" prefix keeps the chain path's
    // one-contract semantics. `schema` is the armed sheet schema snapshot header-keyed row
    // writes validate against.
    pub fn with_contract(
        tx: mpsc::UnboundedSender<ToolEvent>,
        gov: Governance,
        conn: ConnectorManifest,
        schema: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            tx,
            contracts: vec![(
                String::new(),
                Self::make_contract(gov, conn, schema, Arc::default()),
            )],
            state_allowed: None,
            state_name: None,
            peer: None,
            peers: BTreeMap::new(),
            approval_timeout: APPROVAL_TIMEOUT,
        }
    }

    // Enforcing, one contract per connector (the chat surface, and any agent that needs more
    // than one connector). Each entry is (mcp_prefix, governance, manifest); a call resolves to
    // its connector by longest prefix match, and a tool no entry claims is refused. All entries
    // share one run state, so the limits bound the run and not each connector.
    pub fn with_contracts(
        tx: mpsc::UnboundedSender<ToolEvent>,
        entries: Vec<(String, Governance, ConnectorManifest)>,
    ) -> Self {
        let run: Arc<Mutex<RunState>> = Arc::default();
        Self {
            tx,
            contracts: entries
                .into_iter()
                .map(|(prefix, gov, conn)| {
                    (
                        prefix,
                        Self::make_contract(
                            gov,
                            conn,
                            serde_json::Map::new(),
                            run.clone(),
                        ),
                    )
                })
                .collect(),
            state_allowed: None,
            state_name: None,
            peer: None,
            peers: BTreeMap::new(),
            approval_timeout: APPROVAL_TIMEOUT,
        }
    }

    // Longest-prefix resolution of a tool call to its connector's contract.
    fn resolve(&self, tool_name: &str) -> Option<&Arc<Contract>> {
        self.contracts
            .iter()
            .filter(|(prefix, _)| tool_name.starts_with(prefix.as_str()))
            .max_by_key(|(prefix, _)| prefix.len())
            .map(|(_, c)| c)
    }

    // The chain path's single contract. Run-accounting accessors keep single-contract
    // (chain) semantics through this; on the multi-contract chat surface only
    // `aborted()` is consulted.
    fn first(&self) -> Option<&Arc<Contract>> {
        self.contracts.first().map(|(_, c)| c)
    }

    /// The schema map as refreshed by this run's re-syncs, or None when nothing changed.
    /// The chain runtime persists it (the hook never holds the Database).
    pub fn schema_snapshot(
        &self,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let contract = self.first()?;
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
        if let (true, Some(peer)) = (first_resync, self.peer_for(&call.tool)) {
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

    // The fixed agent-facing note on every filtered result. NO count of what was masked: the
    // count itself would leak how much exists outside the armed scope. Identical on the
    // success and fail-closed paths. English constant, like the preambles.
    const SCOPED_NOTE: &str = "scoped to armed resources";

    // A scoped tool's response, executed by the hook itself and filtered to the armed scope.
    // rig cannot rewrite a tool result after the fact, so the hook runs the call through the
    // peer and hands the filtered JSON back as the Skip reason - which the LLM receives as
    // the tool's result. Only called after the gate ALLOWED the call (never executes a Deny),
    // and only ever REMOVES items (never widens). None = this call is not scope-filtered.
    // Any failure (no peer, transport error, unparseable response, missing items path) yields
    // the empty-shape result + note: fail closed, never the raw response.
    async fn scoped_filter(
        &self,
        contract: &Arc<Contract>,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Option<String> {
        let scoping = contract.conn.scoping.as_ref()?;
        let bound = contract.gov.bound_resource.as_ref()?;
        if !bound_pins(Some(bound)) {
            return None;
        }

        // list tool: keep only items whose id is armed.
        if let Some(list) =
            scoping.list.as_ref().filter(|l| l.tool == tool_name)
        {
            let armed = bound_values(bound, &scoping.id_field);
            let filtered = self
                .execute_scoped(tool_name, args)
                .await
                .and_then(|response| {
                    filter_items(response, &list.items_path, |item| {
                        path_str(item, &list.id_key)
                            .is_some_and(|id| armed.contains(&id))
                    })
                })
                .unwrap_or_else(|| empty_scoped(&list.items_path));
            return Some(with_note(filtered, Self::SCOPED_NOTE));
        }

        // tabs tool: mask unarmed tabs, only when every matching entry pins one.
        if let Some(tabs) =
            scoping.tabs.as_ref().filter(|t| t.tool == tool_name)
        {
            let id = args.get(&scoping.id_field).and_then(|v| v.as_str())?;
            let armed_tabs = bound_tabs_for(bound, &scoping.id_field, id)?;
            let filtered = self
                .execute_scoped(tool_name, args)
                .await
                .and_then(|response| {
                    filter_items(response, &tabs.items_path, |item| {
                        path_str(item, &tabs.name_key)
                            .is_some_and(|name| armed_tabs.contains(&name))
                    })
                })
                .unwrap_or_else(|| empty_scoped(&tabs.items_path));
            return Some(with_note(filtered, Self::SCOPED_NOTE));
        }
        None
    }

    // Execute a gate-allowed scoped call through the peer and flatten its result to JSON.
    async fn execute_scoped(
        &self,
        tool: &str,
        args: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        let peer = self.peer_for(tool)?;
        let mut params =
            rmcp::model::CallToolRequestParams::new(tool.to_string());
        if let Some(obj) = args.as_object() {
            params = params.with_arguments(obj.clone());
        }
        let result = peer.call_tool(params).await.ok()?;
        let json = crate::application::connector_module::result_json(&result);
        (!json.is_null()).then_some(json)
    }

    // Attach the MCP server sink so an approved EDITED payload executes deterministically
    // (the hook dials it; rig's Continue can only run the model's original args).
    pub fn with_peer(mut self, peer: rmcp::service::ServerSink) -> Self {
        self.peer = Some(peer);
        self
    }

    // Load the armed sheet schema snapshot row writes are validated against. Applies to every
    // contract: the tools that use it belong to one connector anyway.
    pub fn with_schema(
        self,
        schema: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        for (_, contract) in &self.contracts {
            *contract.schema.lock().expect("schema poisoned") = schema.clone();
        }
        self
    }

    // Name the server behind each tool, so a run with several connectors dials the right one.
    pub fn with_peers(
        mut self,
        peers: Vec<(String, rmcp::service::ServerSink)>,
    ) -> Self {
        self.peers = peers.into_iter().collect();
        self
    }

    // The server to dial for a tool: its owner when known, else the single attached peer.
    fn peer_for(&self, tool: &str) -> Option<&rmcp::service::ServerSink> {
        self.peers.get(tool).or(self.peer.as_ref())
    }

    /// Seconds spent suspended on approval cards this run (summed across contracts).
    pub fn held_seconds(&self) -> u64 {
        self.contracts
            .iter()
            .map(|(_, c)| c.held_secs.load(Ordering::Relaxed))
            .sum()
    }

    /// Whether a rejected/expired card asked the run to conclude instead of advancing
    /// (any contract).
    pub fn aborted(&self) -> bool {
        self.contracts
            .iter()
            .any(|(_, c)| c.abort.load(Ordering::Relaxed))
    }

    // Share this contract's run accounting with a hook scoped to one chain state's allowed tools. The chain
    // runtime builds one base contract, then a scoped hook per state, so budgets and read_before_write carry
    // across states while each state narrows the visible surface.
    pub fn scoped_to(&self, allowed: Vec<String>, state: String) -> Self {
        Self {
            tx: self.tx.clone(),
            contracts: self.contracts.clone(),
            state_allowed: Some(allowed),
            state_name: Some(state),
            peer: self.peer.clone(),
            peers: self.peers.clone(),
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
        if let Some(c) = self.first() {
            c.run.lock().expect("governance run state poisoned").steps += 1;
        }
    }

    // Publish elapsed wall-clock so the gate's `limits.max_run_seconds` ceiling has live data. The chain
    // runtime owns the clock; without it the time bound would never fire. No-op without a contract.
    pub fn set_elapsed(&self, seconds: u64) {
        if let Some(c) = self.first() {
            c.run
                .lock()
                .expect("governance run state poisoned")
                .elapsed_seconds = seconds;
        }
    }

    // Whether ANY bound resource was read this run, for the coarse FSM-level read_before_write guard. The
    // gate enforces the precise per-resource rule; this only gates entry into a write state.
    pub fn read_any(&self) -> bool {
        self.contracts.iter().any(|(_, c)| {
            !c.run
                .lock()
                .expect("governance run state poisoned")
                .read_resources
                .is_empty()
        })
    }

    // Take this run's accumulated tool-call log (proposed args, gate verdict, raw result). The chain runtime
    // drains it after each state to attach ground truth to that state's trace. Empty without a contract.
    pub fn drain_events(&self) -> Vec<String> {
        self.contracts
            .iter()
            .flat_map(|(_, c)| {
                std::mem::take(&mut *c.events.lock().expect("events poisoned"))
            })
            .collect()
    }

    fn record(&self, line: String) {
        if let Some(c) = self.first() {
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
        let peer = self.peer_for(tool).ok_or("no MCP peer attached")?;
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

        // A rejected/expired card ended this run's tool phase: every later call is
        // refused so the model answers with what it has instead of re-proposing.
        if self.aborted() {
            return ToolCallHookAction::Skip {
                reason:
                    "the user ended this run's actions; answer with what you already have"
                        .into(),
            };
        }

        // No contracts = observe-only surface (notes tools only). With contracts, local
        // notes tools pass untouched, and an MCP tool NO contract claims is refused:
        // fail closed at call time, defense in depth behind the surface-build filter.
        if self.contracts.is_empty() {
            return ToolCallHookAction::Continue;
        }
        if NOTES_TOOL_NAMES.contains(&tool_name) {
            return ToolCallHookAction::Continue;
        }
        let Some(contract) = self.resolve(tool_name) else {
            self.record(format!(
                "blocked {tool_name}: no connector contract claims it"
            ));
            return ToolCallHookAction::Skip {
                reason: format!(
                    "tool `{tool_name}` is not governed by any connected connector; do not call it"
                ),
            };
        };
        let contract = contract.clone();
        let contract = &contract;

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
                // A gate-allowed SCOPED tool never runs raw: the hook executes it itself and
                // substitutes the response filtered to the armed scope (Skip-reason = the
                // result the LLM sees). Everything else continues to the normal tool path.
                if let Some(filtered) =
                    self.scoped_filter(contract, tool_name, &call.args).await
                {
                    self.record(format!(
                        "scoped {tool_name} {} -> filtered result",
                        excerpt(args, 160)
                    ));
                    return ToolCallHookAction::Skip { reason: filtered };
                }
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
    notes_tools: NotesTools,
) -> Result<String, crate::application::error::LlmError> {
    crate::application::chat_surface::prompt_chat_agent(
        llm,
        preamble,
        user_message,
        status_tx,
        notes_tools,
    )
    .await
}
