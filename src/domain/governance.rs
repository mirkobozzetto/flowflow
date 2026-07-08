use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

// Layer 1 governance schema (RFC 0010 docs/protocol/03) + the connector data manifest it validates
// against (05). This is the DEVICE copy of the backend's `governance` module: the schema and the gate are
// agreed ONCE so the on-device PromptHook and the backend proxy enforce the exact same contract and cannot
// drift (07, issue #11). The backend-only tail (embedded manifest registry, tools/list filtering, the
// stateless proxy seam) stays on the backend; the device runs the full gate, since it owns the run state.

// Canonical action vocabulary a connector tool maps onto (05). A `type` names the full set; a concrete
// connector implements a subset via `provides`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Search,
    Read,
    Append,
    Update,
    Upsert,
    Create,
    Clear,
    Delete,
}

impl Action {
    pub fn is_write(self) -> bool {
        matches!(
            self,
            Action::Append
                | Action::Update
                | Action::Upsert
                | Action::Create
                | Action::Clear
                | Action::Delete
        )
    }
}

// The risk floor `deny_destructive` keys off. Set from observed behavior, not MCP annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    ReadOnly,
    ReadWrite,
    Destructive,
}

// Per-tool write policy. read_write is the broad non-destructive grant; append_only and upsert are
// narrower restrictions; read_only forbids every write. The `clear`/`delete` ACTIONS are denied by every
// mode here, always. Independently, the destructive RISK floor (`deny_destructive`) refuses any tool the
// connector classifies destructive regardless of its action - the two are separate layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    ReadOnly,
    AppendOnly,
    ReadWrite,
    Upsert,
}

impl Mode {
    // The explicit mode<->action matrix. Adding an Action forces a decision here for every mode.
    //
    //   mode         | search read append update upsert create | clear delete
    //   read_only    |   y      y                              |
    //   append_only  |   y      y     y                        |
    //   upsert       |   y      y                  y           |
    //   read_write   |   y      y     y      y      y      y    |
    //
    // clear/delete fall through to false for every mode (destructive, never granted by mode alone).
    pub fn allows(self, action: Action) -> bool {
        use Action::*;
        match self {
            Mode::ReadOnly => matches!(action, Search | Read),
            Mode::AppendOnly => matches!(action, Search | Read | Append),
            Mode::Upsert => matches!(action, Search | Read | Upsert),
            Mode::ReadWrite => {
                matches!(
                    action,
                    Search | Read | Append | Update | Upsert | Create
                )
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Approval {
    #[default]
    Auto,
    RequireApproval,
}

// Per-column write policy. Identity columns are `key`/`read_only`; the agent never overwrites a populated
// human-owned field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnRole {
    Key,
    ReadOnly,
    Writable,
    AppendOnly,
}

// What the gate does when an upsert key matches more than one row. Only `review` (suspend for a human) is
// specified today; more strategies get added when a connector needs one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnMultipleMatch {
    Review,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Limits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tool_calls: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_run_seconds: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolPolicy {
    pub tool: String,
    pub mode: Mode,
    #[serde(default)]
    pub approval: Approval,
    // upsert match key. Required (non-empty) when mode is upsert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_columns: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_calls_per_run: Option<u32>,
}

fn default_true() -> bool {
    true
}

// The always-enforced safety floor (03). Defaults are the safe choice: a manifest that omits a floor field
// gets it ON, never off. `bound_resource` is connector-specific (e.g. {spreadsheet_id, tab}), so it stays a
// free-form value the gate matches structurally - it is not part of the cross-connector schema.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Governance {
    pub tools: Vec<ToolPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_resource: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_roles: Option<BTreeMap<String, ColumnRole>>,
    #[serde(default = "default_true")]
    pub read_before_write: bool,
    #[serde(default = "default_true")]
    pub deny_destructive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_multiple_match: Option<OnMultipleMatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,
}

// A connector data manifest (connectors/*.json): the raw MCP tools mapped onto (resource, action, risk).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectorManifest {
    pub connector: String,
    #[serde(rename = "type")]
    pub connector_type: String,
    pub server: String,
    pub mcp_prefix: String,
    pub provides: Vec<Action>,
    pub tools: Vec<ConnectorTool>,
}

impl ConnectorManifest {
    pub fn tool(&self, name: &str) -> Option<&ConnectorTool> {
        self.tools.iter().find(|t| t.tool == name)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectorTool {
    pub tool: String,
    pub resource: String,
    pub action: Action,
    pub risk: Risk,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GovernanceError {
    #[error("tool `{tool}` is not in connector `{connector}` manifest")]
    UnknownTool { tool: String, connector: String },
    #[error(
        "tool `{tool}`: mode {mode:?} is not compatible with action {action:?}"
    )]
    ModeActionMismatch {
        tool: String,
        mode: Mode,
        action: Action,
    },
    #[error("tool `{tool}` is destructive and deny_destructive is on")]
    DestructiveDenied { tool: String },
    #[error("tool `{tool}`: mode upsert requires non-empty key_columns")]
    UpsertMissingKeyColumns { tool: String },
    #[error("tool `{tool}`: read_before_write is on but no bound_resource pins the target")]
    ReadBeforeWriteWithoutBound { tool: String },
    #[error("bound_resource field `{field}`: {reason}")]
    MalformedBoundResource { field: String, reason: String },
}

pub fn parse_connector_manifest(
    json: &str,
) -> serde_json::Result<ConnectorManifest> {
    serde_json::from_str(json)
}

pub fn parse_governance(json: &str) -> serde_json::Result<Governance> {
    serde_json::from_str(json)
}

// Whether `bound_resource` actually pins a target: a non-empty object. An empty object pins nothing
// (every call matches), so it does not count as a bound for the read_before_write floor.
fn bound_pins(bound: Option<&serde_json::Value>) -> bool {
    bound
        .and_then(|b| b.as_object())
        .is_some_and(|m| !m.is_empty())
}

// A pinned field value is well-formed when it is a string (scalar match) or a non-empty array of
// strings (membership). Anything else (number, null, bool, object, mixed/empty array) would silently
// deny calls at gate time, so it is rejected at install instead.
fn bound_field_ok(v: &serde_json::Value) -> Result<(), &'static str> {
    match v {
        serde_json::Value::String(_) => Ok(()),
        serde_json::Value::Array(arr) if arr.is_empty() => Err("empty array"),
        serde_json::Value::Array(arr) if arr.iter().all(|e| e.is_string()) => {
            Ok(())
        }
        serde_json::Value::Array(_) => Err("array must contain only strings"),
        _ => Err("must be a string or a non-empty array of strings"),
    }
}

// Validate every governed tool against a connector manifest (run on install, before arming an agent).
// Collects ALL violations rather than failing at the first. Same call shape as the backend so they cannot
// drift.
pub fn validate_governance(
    gov: &Governance,
    conn: &ConnectorManifest,
) -> Result<(), Vec<GovernanceError>> {
    let mut errors = Vec::new();

    // Structural check of the bound: a present bound must be a non-empty object whose fields are each a
    // string or a non-empty all-string array. A scalar/empty bound is caught here, not silently at runtime.
    if let Some(bound) = &gov.bound_resource {
        match bound.as_object() {
            None => errors.push(GovernanceError::MalformedBoundResource {
                field: "<root>".into(),
                reason: "bound_resource must be an object".into(),
            }),
            Some(map) if map.is_empty() => {
                errors.push(GovernanceError::MalformedBoundResource {
                    field: "<root>".into(),
                    reason: "bound_resource must pin at least one field".into(),
                })
            }
            Some(map) => {
                for (k, v) in map {
                    if let Err(reason) = bound_field_ok(v) {
                        errors.push(GovernanceError::MalformedBoundResource {
                            field: k.clone(),
                            reason: reason.into(),
                        });
                    }
                }
            }
        }
    }

    for tp in &gov.tools {
        let Some(ct) = conn.tool(&tp.tool) else {
            errors.push(GovernanceError::UnknownTool {
                tool: tp.tool.clone(),
                connector: conn.connector.clone(),
            });
            continue;
        };

        if gov.deny_destructive && ct.risk == Risk::Destructive {
            errors.push(GovernanceError::DestructiveDenied {
                tool: tp.tool.clone(),
            });
            continue;
        }

        if !tp.mode.allows(ct.action) {
            errors.push(GovernanceError::ModeActionMismatch {
                tool: tp.tool.clone(),
                mode: tp.mode,
                action: ct.action,
            });
        }

        if tp.mode == Mode::Upsert
            && tp.key_columns.as_ref().is_none_or(|k| k.is_empty())
        {
            errors.push(GovernanceError::UpsertMissingKeyColumns {
                tool: tp.tool.clone(),
            });
        }

        // A genuine write grant under read_before_write needs a pinned target: without one, the
        // global read flag means only "some read happened", not "the agent read what it writes". Pinning
        // the resource forces every read to target it, so the floor matches the per-resource intent.
        if ct.action.is_write()
            && tp.mode.allows(ct.action)
            && gov.read_before_write
            && !bound_pins(gov.bound_resource.as_ref())
        {
            errors.push(GovernanceError::ReadBeforeWriteWithoutBound {
                tool: tp.tool.clone(),
            });
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ---- the propose -> verify -> commit gate (03; applied on-device in the PromptHook, 07/09) ----

// One proposed governed tool call. `args` is the raw MCP payload; `touched_columns` is the semantic set a
// connector adapter extracts from those args - the gate never parses raw args for column meaning, so it
// stays connector-agnostic. Empty `touched_columns` = the call touches no governed column (e.g. a read).
#[derive(Debug, Clone)]
pub struct ProposedCall {
    pub tool: String,
    pub args: serde_json::Value,
    pub touched_columns: Vec<String>,
}

impl ProposedCall {
    pub fn new(tool: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            tool: tool.into(),
            args,
            touched_columns: Vec::new(),
        }
    }

    pub fn with_columns(
        mut self,
        cols: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.touched_columns = cols.into_iter().map(Into::into).collect();
        self
    }
}

// Mutable per-run accounting. The gate VERIFIES against it and, only on Allow, COMMITS to it (advances
// budgets, records the read). `steps`/`elapsed_seconds` are owned by the caller (the chain runtime) and
// updated there; the gate only reads them against `limits`. A Hold commits nothing.
#[derive(Debug, Clone, Default)]
pub struct RunState {
    pub tool_calls: u32,
    pub per_tool: BTreeMap<String, u32>,
    pub steps: u32,
    pub elapsed_seconds: u64,
    // The resources read this run, keyed by `resource_key`. read_before_write checks the write's own
    // resource is in here, so a read of one bound sheet never satisfies a write to another.
    pub read_resources: BTreeSet<String>,
    // One-shot approval grants by call fingerprint: inserted when the user approves a held
    // proposal, CONSUMED by the gate pass that executes it. One approval = one execution.
    pub approvals: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    // A legal require_approval write, suspended until the user decides. Only a call every
    // other check would Allow can be held: a card never shows a call the gate refuses.
    Hold(Proposal),
    Deny(DenyReason),
}

impl Decision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allow)
    }
}

// The domain-pure description of a suspended call. `fingerprint` is its approval identity:
// canonical(tool + args), so an edited payload is a NEW proposal and a stale approval can
// never authorize different bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub tool: String,
    pub action: Action,
    pub args: serde_json::Value,
    pub fingerprint: String,
}

/// The approval identity of a call: tool + canonically-serialized args (sorted keys, compact),
/// so key order or whitespace can never mint a second identity for the same payload.
pub fn call_fingerprint(tool: &str, args: &serde_json::Value) -> String {
    format!(
        "{tool}:{}",
        crate::domain::agent_manifest::canonical_json(args)
    )
}

// The single structured reason a call was denied: surfaced to the model (as the Skip reason) so it
// self-corrects. Serializable so it can be logged. Never silent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(tag = "deny", rename_all = "snake_case")]
pub enum DenyReason {
    #[error("tool `{tool}` is not in the agent's governed tools[]")]
    NotAllowed { tool: String },
    #[error("tool `{tool}` is not in connector `{connector}` manifest")]
    UnknownTool { tool: String, connector: String },
    #[error("tool `{tool}`: mode {mode:?} cannot perform action {action:?}")]
    ModeActionMismatch {
        tool: String,
        mode: Mode,
        action: Action,
    },
    #[error("tool `{tool}`: column `{column}` is {role:?}, not writable")]
    ColumnNotWritable {
        tool: String,
        column: String,
        role: ColumnRole,
    },
    #[error(
        "tool `{tool}`: column `{column}` is not declared in column_roles"
    )]
    UngovernedColumn { tool: String, column: String },
    #[error("tool `{tool}`: call does not target the bound resource")]
    OutOfBoundResource { tool: String },
    #[error("tool `{tool}`: write attempted before reading the bound resource this run")]
    ReadBeforeWrite { tool: String },
    #[error("tool `{tool}` is destructive and deny_destructive is on")]
    Destructive { tool: String },
    #[error("tool `{tool}`: per-run call budget reached (max {max})")]
    PerToolBudget { tool: String, max: u32 },
    #[error("run tool-call budget reached (max {max})")]
    ToolCallBudget { max: u32 },
    #[error("run step budget reached (max {max})")]
    StepBudget { max: u32 },
    #[error("run time budget reached (max {max_seconds}s)")]
    TimeBudget { max_seconds: u64 },
}

// bound_resource match = the call's args CONTAIN every pinned field. A pinned scalar must equal the arg;
// a pinned array means "one of" - the arg must be a member. Connector-agnostic: it pins the target(s)
// without knowing the connector's arg schema. A bound that is not an object pins nothing.
fn args_match_bound(
    args: &serde_json::Value,
    bound: &serde_json::Value,
) -> bool {
    match bound.as_object() {
        None => true,
        Some(pinned) => match args.as_object() {
            None => false,
            Some(a) => pinned.iter().all(|(k, v)| match (a.get(k), v) {
                (Some(av), serde_json::Value::Array(allowed)) => {
                    allowed.contains(av)
                }
                (Some(av), scalar) => av == scalar,
                (None, _) => false,
            }),
        },
    }
}

// The canonical identity of the resource a call targets: the bound field NAMES pulled out of the call's
// args, sorted (BTreeMap), serialized compact. Sorting closes a read-skip via reordered keys. Used to
// track read_before_write per resource, so reading sheet X never unlocks a write to a sibling sheet Y.
// A None/non-object bound yields the empty key, collapsing to the global "any read" behavior.
fn resource_key(
    args: &serde_json::Value,
    bound: Option<&serde_json::Value>,
) -> String {
    let (Some(args), Some(bound)) =
        (args.as_object(), bound.and_then(|b| b.as_object()))
    else {
        return String::new();
    };
    let mut key = BTreeMap::new();
    for name in bound.keys() {
        if let Some(val) = args.get(name) {
            key.insert(name.clone(), val.clone());
        }
    }
    serde_json::to_string(&key).unwrap_or_default()
}

// ---- per-call structural checks (shared by the gate; mirror the backend so the two seams cannot drift) ----

fn resolve<'a>(
    gov: &'a Governance,
    conn: &'a ConnectorManifest,
    tool: &str,
) -> Result<(&'a ToolPolicy, &'a ConnectorTool), DenyReason> {
    let tp = gov
        .tools
        .iter()
        .find(|t| t.tool == tool)
        .ok_or_else(|| DenyReason::NotAllowed { tool: tool.into() })?;
    let ct = conn.tool(tool).ok_or_else(|| DenyReason::UnknownTool {
        tool: tool.into(),
        connector: conn.connector.clone(),
    })?;
    Ok((tp, ct))
}

fn check_mode_action(
    tp: &ToolPolicy,
    ct: &ConnectorTool,
    tool: &str,
) -> Option<DenyReason> {
    (!tp.mode.allows(ct.action)).then(|| DenyReason::ModeActionMismatch {
        tool: tool.into(),
        mode: tp.mode,
        action: ct.action,
    })
}

fn check_column_roles(
    gov: &Governance,
    ct: &ConnectorTool,
    call: &ProposedCall,
) -> Option<DenyReason> {
    if ct.action.is_write() {
        if let Some(roles) = &gov.column_roles {
            for col in &call.touched_columns {
                match roles.get(col) {
                    None => {
                        return Some(DenyReason::UngovernedColumn {
                            tool: call.tool.clone(),
                            column: col.clone(),
                        });
                    }
                    Some(ColumnRole::Writable) => {}
                    Some(ColumnRole::AppendOnly)
                        if ct.action == Action::Append => {}
                    Some(&role) => {
                        return Some(DenyReason::ColumnNotWritable {
                            tool: call.tool.clone(),
                            column: col.clone(),
                            role,
                        });
                    }
                }
            }
        }
    }
    None
}

fn check_bound(
    gov: &Governance,
    ct: &ConnectorTool,
    call: &ProposedCall,
) -> Option<DenyReason> {
    (ct.action != Action::Search
        && gov
            .bound_resource
            .as_ref()
            .is_some_and(|bound| !args_match_bound(&call.args, bound)))
    .then(|| DenyReason::OutOfBoundResource {
        tool: call.tool.clone(),
    })
}

fn check_destructive(
    gov: &Governance,
    ct: &ConnectorTool,
    tool: &str,
) -> Option<DenyReason> {
    (gov.deny_destructive && ct.risk == Risk::Destructive)
        .then(|| DenyReason::Destructive { tool: tool.into() })
}

// propose -> verify -> commit. Checks run in a FIXED order (each Deny short-circuits with one reason):
// allowlist -> mode/column_roles -> bound_resource -> read_before_write -> deny_destructive -> limits.
// On Allow it commits: advances the call budgets and, for a read, records that the bound resource was read.
// The device owns the run state, so it enforces the FULL gate including the run-stateful checks.
pub fn gate(
    gov: &Governance,
    conn: &ConnectorManifest,
    call: &ProposedCall,
    run: &mut RunState,
) -> Decision {
    let (tp, ct) = match resolve(gov, conn, &call.tool) {
        Ok(pair) => pair,
        Err(reason) => return Decision::Deny(reason),
    };
    if let Some(reason) = check_mode_action(tp, ct, &call.tool) {
        return Decision::Deny(reason);
    }
    if let Some(reason) = check_column_roles(gov, ct, call) {
        return Decision::Deny(reason);
    }
    if let Some(reason) = check_bound(gov, ct, call) {
        return Decision::Deny(reason);
    }

    if ct.action.is_write()
        && gov.read_before_write
        && !run
            .read_resources
            .contains(&resource_key(&call.args, gov.bound_resource.as_ref()))
    {
        return Decision::Deny(DenyReason::ReadBeforeWrite {
            tool: call.tool.clone(),
        });
    }

    if let Some(reason) = check_destructive(gov, ct, &call.tool) {
        return Decision::Deny(reason);
    }

    if let Some(max) = tp.max_calls_per_run {
        if run.per_tool.get(&call.tool).copied().unwrap_or(0) >= max {
            return Decision::Deny(DenyReason::PerToolBudget {
                tool: call.tool.clone(),
                max,
            });
        }
    }
    if let Some(limits) = &gov.limits {
        if let Some(max) = limits.max_tool_calls {
            if run.tool_calls >= max {
                return Decision::Deny(DenyReason::ToolCallBudget { max });
            }
        }
        if let Some(max) = limits.max_steps {
            if run.steps >= max {
                return Decision::Deny(DenyReason::StepBudget { max });
            }
        }
        if let Some(max) = limits.max_run_seconds {
            if run.elapsed_seconds >= max as u64 {
                return Decision::Deny(DenyReason::TimeBudget {
                    max_seconds: max as u64,
                });
            }
        }
    }

    // Approval floor, last: only a call every check above allows can be held, so the card
    // never shows a call the gate refuses. The grant is one-shot: consumed by THIS pass, so
    // one approval executes exactly one call and edited args (a new fingerprint) re-hold.
    if tp.approval == Approval::RequireApproval && ct.action.is_write() {
        let fp = call_fingerprint(&call.tool, &call.args);
        if !run.approvals.remove(&fp) {
            return Decision::Hold(Proposal {
                tool: call.tool.clone(),
                action: ct.action,
                args: call.args.clone(),
                fingerprint: fp,
            });
        }
    }

    run.tool_calls += 1;
    *run.per_tool.entry(call.tool.clone()).or_insert(0) += 1;
    if ct.action == Action::Read {
        run.read_resources
            .insert(resource_key(&call.args, gov.bound_resource.as_ref()));
    }
    Decision::Allow
}
