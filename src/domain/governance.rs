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

// What upsert_rows does when a key_columns value matches more than one row in the live sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnMultipleMatch {
    First,
    Last,
    Error,
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
    // How this connector's responses are scoped to armed resources. Absent = the hook
    // cannot filter, so every Search tool is refused once a bound exists (fail closed).
    // A malformed block fails the whole manifest parse, which drops the connector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scoping: Option<Scoping>,
}

// Declares which tools return lists the hook must filter to the armed ids, and where the
// ids live in their responses. `id_field` names the bound/args field carrying the resource id.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scoping {
    pub id_field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<ListScoping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<TabsScoping>,
}

// The listing tool: `items_path` locates the response array, `id_key` the resource id per item.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListScoping {
    pub tool: String,
    pub items_path: String,
    pub id_key: String,
}

// The per-resource read whose response enumerates tabs: `name_key` is a dotted path to each
// item's tab name, matched against the armed entries' pinned `sheet`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TabsScoping {
    pub tool: String,
    pub items_path: String,
    pub name_key: String,
}

impl Scoping {
    /// Whether `tool` is declared as a scoped (hook-filtered) tool.
    pub fn declares(&self, tool: &str) -> bool {
        self.list.as_ref().is_some_and(|l| l.tool == tool)
            || self.tabs.as_ref().is_some_and(|t| t.tool == tool)
    }
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

// The binding v2 field that pins one tab of a resource. A read that does not address a tab
// still passes on the other pins (the hook filters tabs out of the response); a write MUST
// address it, via a dedicated arg or an A1 "Tab!A1" prefix.
const TAB_PIN_FIELD: &str = "sheet";

// The bound as OR-of-AND entries: object = one v1 entry (array values = one-of membership),
// array = v2 entries. None = a shape that pins nothing parseable (fail closed downstream).
fn bound_entries(
    bound: &serde_json::Value,
) -> Option<Vec<&serde_json::Map<String, serde_json::Value>>> {
    match bound {
        serde_json::Value::Object(o) => Some(vec![o]),
        serde_json::Value::Array(arr) => {
            let entries: Vec<_> =
                arr.iter().filter_map(|e| e.as_object()).collect();
            (entries.len() == arr.len()).then_some(entries)
        }
        _ => None,
    }
}

/// Whether `bound_resource` actually pins a target: a non-empty object, or a v2 array with at
/// least one non-empty entry. An empty object pins nothing (every call matches), so it does not
/// count as a bound for the read_before_write floor. An UNPARSEABLE bound counts as pinning:
/// treating it as open would silently widen the scope on a corrupt binding.
pub fn bound_pins(bound: Option<&serde_json::Value>) -> bool {
    let Some(b) = bound else { return false };
    match bound_entries(b) {
        Some(entries) => entries.iter().any(|e| !e.is_empty()),
        None => true,
    }
}

// The tab a call addresses: the dedicated arg, else the A1 prefix of a `range`/`cell` value
// ("Clients!B2", "'My Tab'!A1:C3"). None = the call does not address a tab.
fn call_sheet(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    if let Some(v) = args.get(TAB_PIN_FIELD) {
        return Some(v.clone());
    }
    for key in ["range", "cell"] {
        if let Some(s) = args.get(key).and_then(|v| v.as_str()) {
            if let Some(name) = a1_sheet_prefix(s) {
                return Some(serde_json::Value::String(name));
            }
        }
    }
    None
}

// The sheet name before '!' in an A1 reference, unquoting `'My Sheet'` and the `''` escape.
fn a1_sheet_prefix(a1: &str) -> Option<String> {
    if let Some(rest) = a1.strip_prefix('\'') {
        // Quoted form: the name runs to the closing quote ('' escapes a literal quote).
        let mut name = String::new();
        let mut chars = rest.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    name.push('\'');
                } else {
                    return chars
                        .next()
                        .is_some_and(|n| n == '!')
                        .then_some(name);
                }
            } else {
                name.push(c);
            }
        }
        None
    } else {
        let bang = a1.find('!')?;
        (bang > 0).then(|| a1[..bang].to_string())
    }
}

// One entry matches when every pinned field agrees with the call's args. A pinned field the
// call does not carry is fatal for a WRITE (a write must address every pin - the A1 prefix
// counts as the tab arg) but tolerated on a read/search for the TAB pin only: the read is
// gated on the ids it carries and the hook filters tabs out of the response.
fn entry_matches(
    entry: &serde_json::Map<String, serde_json::Value>,
    args: &serde_json::Value,
    is_write: bool,
) -> bool {
    let Some(a) = args.as_object() else {
        return entry.is_empty();
    };
    entry.iter().all(|(k, pinned)| {
        let arg = if k == TAB_PIN_FIELD {
            call_sheet(a)
        } else {
            a.get(k).cloned()
        };
        match (arg, pinned) {
            (Some(av), serde_json::Value::Array(allowed)) => {
                allowed.contains(&av)
            }
            (Some(av), scalar) => &av == scalar,
            (None, _) => !is_write && k == TAB_PIN_FIELD,
        }
    })
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

    // Structural check of the bound: a present bound must be a non-empty object (v1) or a
    // non-empty array of non-empty objects (v2), each field a string or a non-empty all-string
    // array. A scalar/empty bound is caught here, not silently at runtime.
    if let Some(bound) = &gov.bound_resource {
        let check_entry =
            |map: &serde_json::Map<String, serde_json::Value>,
             errors: &mut Vec<GovernanceError>| {
                if map.is_empty() {
                    errors.push(GovernanceError::MalformedBoundResource {
                        field: "<root>".into(),
                        reason: "bound entry must pin at least one field"
                            .into(),
                    });
                }
                for (k, v) in map {
                    if let Err(reason) = bound_field_ok(v) {
                        errors.push(GovernanceError::MalformedBoundResource {
                            field: k.clone(),
                            reason: reason.into(),
                        });
                    }
                }
            };
        match bound {
            serde_json::Value::Object(map) => check_entry(map, &mut errors),
            serde_json::Value::Array(arr) if arr.is_empty() => {
                errors.push(GovernanceError::MalformedBoundResource {
                    field: "<root>".into(),
                    reason: "bound_resource must pin at least one entry".into(),
                })
            }
            serde_json::Value::Array(arr) => {
                for e in arr {
                    match e.as_object() {
                        Some(map) => check_entry(map, &mut errors),
                        None => errors.push(
                            GovernanceError::MalformedBoundResource {
                                field: "<root>".into(),
                                reason: "bound entry must be an object".into(),
                            },
                        ),
                    }
                }
            }
            _ => errors.push(GovernanceError::MalformedBoundResource {
                field: "<root>".into(),
                reason:
                    "bound_resource must be an object or an array of objects"
                        .into(),
            }),
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
    #[error("tool `{tool}`: listing is scoped to armed resources and this tool's response cannot be filtered")]
    UnscopedSearch { tool: String },
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
    #[error(
        "tool `{tool}`: column `{column}` is not in the sheet's header row (headers: {headers:?})"
    )]
    SchemaMismatch {
        tool: String,
        column: String,
        headers: Vec<String>,
    },
    #[error("tool `{tool}`: the target tab has no header row yet; add column names to row 1 first")]
    NoHeaderRow { tool: String },
    #[error("tool `{tool}`: the armed sheet's schema is not captured; re-arm the sheet")]
    SchemaUnknown { tool: String },
    #[error("tool `{tool}`: key_columns do not match the agent's authorized key_columns")]
    KeyColumnsMismatch { tool: String },
    #[error("tool `{tool}`: duplicate key within the batch ({key})")]
    DuplicateKeyInBatch { tool: String, key: String },
}

// bound_resource match = at least ONE entry agrees with the call (OR of AND). Connector-agnostic:
// it pins the target(s) without knowing the connector's arg schema. An unparseable bound matches
// nothing (fail closed: a corrupt binding must never widen the scope).
fn args_match_bound(
    args: &serde_json::Value,
    bound: &serde_json::Value,
    is_write: bool,
) -> bool {
    match bound_entries(bound) {
        Some(entries) => entries
            .iter()
            .any(|entry| entry_matches(entry, args, is_write)),
        None => false,
    }
}

// The canonical identity of one matched entry for a call: the entry's pinned field NAMES with the
// call's values where the call carries them, the entry's own pin otherwise; sorted (BTreeMap),
// serialized compact. Sorting closes a read-skip via reordered keys.
fn entry_key(
    entry: &serde_json::Map<String, serde_json::Value>,
    args: &serde_json::Value,
) -> String {
    let a = args.as_object();
    let mut key = BTreeMap::new();
    for (name, pinned) in entry {
        let val = a
            .and_then(|m| {
                if name == TAB_PIN_FIELD {
                    call_sheet(m)
                } else {
                    m.get(name).cloned()
                }
            })
            .unwrap_or_else(|| pinned.clone());
        key.insert(name.clone(), val);
    }
    serde_json::to_string(&key).unwrap_or_default()
}

/// The armed values of one bound field across every entry, for the hook's response filter
/// (e.g. the armed spreadsheet ids). Membership arrays contribute each member.
pub fn bound_values(
    bound: &serde_json::Value,
    field: &str,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in bound_entries(bound).unwrap_or_default() {
        match entry.get(field) {
            Some(serde_json::Value::String(s)) => {
                out.insert(s.clone());
            }
            Some(serde_json::Value::Array(arr)) => out.extend(
                arr.iter().filter_map(|v| v.as_str().map(String::from)),
            ),
            _ => {}
        }
    }
    out
}

/// The pinned tabs for one armed resource id: Some(tabs) when EVERY entry matching the id pins
/// a tab (the response filter masks the others), None when any matching entry arms the whole
/// resource - or when the id is not armed at all (the gate already refused that call).
pub fn bound_tabs_for(
    bound: &serde_json::Value,
    id_field: &str,
    id: &str,
) -> Option<BTreeSet<String>> {
    let mut tabs = BTreeSet::new();
    let mut matched = false;
    for entry in bound_entries(bound).unwrap_or_default() {
        let id_matches = match entry.get(id_field) {
            Some(serde_json::Value::String(s)) => s == id,
            Some(serde_json::Value::Array(arr)) => {
                arr.iter().any(|v| v.as_str() == Some(id))
            }
            _ => false,
        };
        if !id_matches {
            continue;
        }
        matched = true;
        match entry.get(TAB_PIN_FIELD).and_then(|v| v.as_str()) {
            Some(tab) => {
                tabs.insert(tab.to_string());
            }
            None => return None,
        }
    }
    matched.then_some(tabs)
}

// The resource identities a call targets: one key per matched entry. read_before_write tracks
// these, so reading armed tab X never unlocks a write to armed tab Y (a whole-spreadsheet read
// matches - and unlocks - every entry it covers). A None/unparseable bound yields the single
// empty key, collapsing to the global "any read" behavior (writes were already denied upstream
// for the unparseable case).
fn resource_keys(
    args: &serde_json::Value,
    bound: Option<&serde_json::Value>,
    is_write: bool,
) -> Vec<String> {
    let Some(entries) = bound.and_then(bound_entries) else {
        return vec![String::new()];
    };
    let keys: Vec<String> = entries
        .iter()
        .filter(|e| entry_matches(e, args, is_write))
        .map(|e| entry_key(e, args))
        .collect();
    if keys.is_empty() {
        vec![String::new()]
    } else {
        keys
    }
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
    conn: &ConnectorManifest,
    ct: &ConnectorTool,
    call: &ProposedCall,
) -> Option<DenyReason> {
    let bound = gov.bound_resource.as_ref()?;
    if ct.action == Action::Search {
        // Search takes no resource id, so it cannot be arg-gated. Unpinned bound = free
        // discovery (arm-time listing). Pinned: only a Search the manifest's `scoping`
        // declares stays callable - the hook executes it and filters the response to the
        // armed ids. Undeclared = the response cannot be scoped = refused (fail closed).
        let exempt = !bound_pins(gov.bound_resource.as_ref())
            || conn
                .scoping
                .as_ref()
                .is_some_and(|s| s.declares(&call.tool));
        return (!exempt).then(|| DenyReason::UnscopedSearch {
            tool: call.tool.clone(),
        });
    }
    (!args_match_bound(&call.args, bound, ct.action.is_write())).then(|| {
        DenyReason::OutOfBoundResource {
            tool: call.tool.clone(),
        }
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
    if let Some(reason) = check_bound(gov, conn, ct, call) {
        return Decision::Deny(reason);
    }

    // The write's own resource(s) must have been read this run: ANY matched entry's key
    // suffices (a covering read inserted every entry it matched).
    if ct.action.is_write()
        && gov.read_before_write
        && !resource_keys(&call.args, gov.bound_resource.as_ref(), true)
            .iter()
            .any(|k| run.read_resources.contains(k))
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
        // A read unlocks EVERY entry it matched: a whole-spreadsheet read covers each of
        // its armed tabs, while a tab-addressed read unlocks that tab alone.
        for key in resource_keys(&call.args, gov.bound_resource.as_ref(), false)
        {
            run.read_resources.insert(key);
        }
    }
    Decision::Allow
}

// ---- header-keyed row tools: shared column/key validation ----
//
// This block is byte-identical in the device and backend copies of this module, and pinned by the
// shared conformance vectors: it is gate logic, drift here means the two seams disagree on what a
// row write may touch.

// The two header-keyed write tools the Sheets proxy answers synthetically. Their `rows` arg is a
// list of {header: value} dicts, so the touched columns are extractable connector-agnostically.
pub fn is_row_tool(tool: &str) -> bool {
    tool == "google_sheets_append_rows" || tool == "google_sheets_upsert_rows"
}

// The semantic columns a row-tool call touches: the union of every row dict's keys, sorted and
// deduplicated. Empty for a non-row tool or a malformed payload (the shape checks deny elsewhere).
pub fn row_tool_touched_columns(args: &serde_json::Value) -> Vec<String> {
    let mut cols: BTreeSet<String> = BTreeSet::new();
    if let Some(rows) = args.get("rows").and_then(|r| r.as_array()) {
        for row in rows {
            if let Some(obj) = row.as_object() {
                cols.extend(obj.keys().cloned());
            }
        }
    }
    cols.into_iter().collect()
}

// The requested upsert key columns, if present and well-formed.
fn row_tool_key_columns(args: &serde_json::Value) -> Option<Vec<String>> {
    args.get("key_columns")?.as_array().map(|a| {
        a.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}

// Validate a row-tool call against the captured header schema and the agent's key policy.
// `headers`: Some(list) = the captured header row for the targeted tab; Some(empty) = captured
// and the tab has no header row; None = no schema available at this seam (the device re-syncs
// then denies; the backend has no schema store and its executor re-reads live headers anyway,
// so header membership is skipped there and only the key/batch rules run).
pub fn validate_row_batch(
    gov: &Governance,
    tool: &str,
    args: &serde_json::Value,
    headers: Option<&[String]>,
) -> Option<DenyReason> {
    if !is_row_tool(tool) {
        return None;
    }
    if let Some(h) = headers {
        if h.is_empty() {
            return Some(DenyReason::NoHeaderRow { tool: tool.into() });
        }
        for col in row_tool_touched_columns(args) {
            if !h.iter().any(|x| x == &col) {
                return Some(DenyReason::SchemaMismatch {
                    tool: tool.into(),
                    column: col,
                    headers: h.to_vec(),
                });
            }
        }
    }
    if tool == "google_sheets_upsert_rows" {
        let Some(requested) = row_tool_key_columns(args) else {
            return Some(DenyReason::KeyColumnsMismatch { tool: tool.into() });
        };
        if requested.is_empty() {
            return Some(DenyReason::KeyColumnsMismatch { tool: tool.into() });
        }
        let authorized = gov
            .tools
            .iter()
            .find(|tp| tp.tool == tool)
            .and_then(|tp| tp.key_columns.as_ref());
        if authorized != Some(&requested) {
            return Some(DenyReason::KeyColumnsMismatch { tool: tool.into() });
        }
        if let Some(h) = headers {
            for kc in &requested {
                if !h.iter().any(|x| x == kc) {
                    return Some(DenyReason::SchemaMismatch {
                        tool: tool.into(),
                        column: kc.clone(),
                        headers: h.to_vec(),
                    });
                }
            }
        }
        // Intra-batch duplicate: two rows carrying the same key value(s) make the batch's own
        // outcome order-dependent, so it is refused deterministically before anything runs.
        let mut seen: BTreeSet<String> = BTreeSet::new();
        if let Some(rows) = args.get("rows").and_then(|r| r.as_array()) {
            for row in rows {
                let key: Vec<String> = requested
                    .iter()
                    .map(|kc| {
                        row.get(kc)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default()
                    })
                    .collect();
                let key = key.join("\u{1f}");
                if !seen.insert(key.clone()) {
                    return Some(DenyReason::DuplicateKeyInBatch {
                        tool: tool.into(),
                        key: key.replace('\u{1f}', ", "),
                    });
                }
            }
        }
    }
    None
}
