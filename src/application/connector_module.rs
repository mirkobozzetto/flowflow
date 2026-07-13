// Runs one installed, pinned agent end to end: verifies + pins a signed package on first use, builds
// the agent from its manifest (governance, chain, preamble, the real bound_resource), dials the
// agent-scoped MCP route with x-agent-id, and enforces the governance gate on-device. The contract is
// no longer hardcoded here - it comes from the manifest the builder resolves.

use crate::application::agent_builder::{build_agent, merge_bound, BuiltAgent};
use crate::application::chain::{run_chain, ChainOutcome};
use crate::domain::agent_manifest::{digest_of_stored, parse_manifest};
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::mcp::McpRegistry;
use crate::infrastructure::persistence::Database;
use rmcp::model::CallToolRequestParams;

// The arm-time tool: lists the connector's spreadsheets so the user can pick one. `search` on the
// resource, so the governance gate never bound-checks it - listing works before anything is armed.
const LIST_TOOL: &str = "google_sheets_list_spreadsheets";

// The agent the arm/bind path installs. Must equal the manifest `id` AND the backend catalog id,
// since it travels as x-agent-id; an id the backend has not granted is rejected 403 at the proxy
// and at the package fetch.
pub const FIXTURE_AGENT_ID: &str = "agent-crm";

// A signed agent package, kept as the test golden vector that exercises verify -> build -> gate without
// a backend. It is NO LONGER the runtime install source: the device now fetches the package from
// `GET /v1/agents/{id}/package` and pins that. The signature is over the canonical digest of the
// manifest, made offline with the dev admin key whose public half is pinned in `ADMIN_PUBKEY`.
// `bound_resource.spreadsheet_id` is a placeholder until the user arms a real sheet; off-bound
// reads/writes are refused until then.
pub const FIXTURE_PACKAGE: &str = r#"{
  "manifest": {
    "schema_version": "1",
    "id": "agent-crm-sync",
    "version": "1.0.0",
    "name": "CRM Sync",
    "description": "Use when the user wants to read or update their client/prospect spreadsheet. Do NOT use for general questions or the calendar.",
    "author": "flowflow-admin",
    "alias": "synchro-clients",
    "model": "gpt-5.4-mini",
    "temperature": 0.1,
    "required_connectors": [
      { "type": "tabular_store", "capabilities": ["search", "read", "update"] }
    ],
    "system_prompt": "You keep the user's client spreadsheet tidy. Read before you write, and act only on the bound sheet.",
    "governance": {
      "tools": [
        { "tool": "google_sheets_list_spreadsheets", "mode": "read_only" },
        { "tool": "google_sheets_get_spreadsheet",   "mode": "read_only" },
        { "tool": "google_sheets_write_to_cell",     "mode": "read_write" }
      ],
      "bound_resource": { "spreadsheet_id": "bound-at-install" },
      "read_before_write": true,
      "deny_destructive": true,
      "limits": { "max_steps": 6, "max_tool_calls": 30 }
    },
    "orchestration": {
      "chains": {
        "sync": {
          "initial": "find",
          "states": {
            "find":   { "allowed_tools": ["google_sheets_list_spreadsheets"], "on_done": "read" },
            "read":   { "allowed_tools": ["google_sheets_get_spreadsheet"], "on_done": "act" },
            "act":    { "allowed_tools": ["google_sheets_write_to_cell"], "guard": "read_before_write", "on_done": "answer" },
            "answer": { "terminal": true }
          }
        }
      }
    }
  },
  "content_digest": "sha256:98cbcbe81a0b5e614944f1c6a1682ee1b75ff425752e46ebfc23c75fcfac3557",
  "signature": "ed25519:tatFcv1stw3Hrhj3UyWtpF9ZVXU6nm8ipkQ/uU6LIYzP0uy/W3mbdjEboaEOj55t7j51ZRjFGsvVVpQa9MnwAQ==",
  "signer_key_id": "dev-admin",
  "status": "published"
}"#;

/// Run one named chain of any installed agent through the FSM runtime (activation by alias,
/// trigger words, or the + palette). Kill switch checked on every run via ensure_installed.
/// `events` reaches the chat status UI: tool start/finish and approval proposals.
pub async fn run_agent_chain(
    db: &Database,
    agent_id: &str,
    chain_name: &str,
    goal: &str,
    events: tokio::sync::mpsc::UnboundedSender<
        crate::application::tools::ToolEvent,
    >,
) -> Result<ChainOutcome, String> {
    crate::application::agent_directory::ensure_installed(db, agent_id).await?;
    let built = load_built(db, agent_id)?;
    let chain = built
        .chains
        .get(chain_name)
        .ok_or_else(|| format!("manifest has no `{chain_name}` chain"))?;
    let mut outcome = run_chain(db, chain, &built, goal, events)
        .await
        .map_err(|e| e.to_string())?;
    outcome.final_text =
        repair_sheet_links(&format_outcome(&outcome), &bound_ids(db));
    Ok(outcome)
}

/// The answer step is LLM-composed markdown; models garble long spreadsheet ids when writing
/// links, producing "file does not exist" URLs. The gate only lets the agent touch ARMED sheets,
/// so any sheets link whose id is not armed is provably wrong: point it at the single armed sheet
/// when unambiguous, else drop the link and keep its label.
pub fn repair_sheet_links(text: &str, valid_ids: &[String]) -> String {
    const PREFIX: &str = "https://docs.google.com/spreadsheets/d/";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(PREFIX) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + PREFIX.len()..];
        let id_len = after
            .find(|c: char| {
                !(c.is_ascii_alphanumeric() || c == '-' || c == '_')
            })
            .unwrap_or(after.len());
        let (id, tail) = after.split_at(id_len);
        if valid_ids.iter().any(|v| v == id) {
            out.push_str(PREFIX);
            out.push_str(id);
        } else if valid_ids.len() == 1 {
            out.push_str(PREFIX);
            out.push_str(&valid_ids[0]);
        } else if let Some(stripped) = strip_link_around(&mut out, tail) {
            rest = stripped;
            continue;
        } else {
            out.push_str(PREFIX);
            out.push_str(id);
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

// `out` ends with "[label](" and `tail` starts at the URL remainder: rewind the markdown link,
// keep the label, swallow up to the closing paren. Returns the new remainder, or None when the
// surrounding text is not a markdown link (bare URL with no armed sheet to point at - left as-is).
fn strip_link_around<'a>(out: &mut String, tail: &'a str) -> Option<&'a str> {
    let open = out.rfind("](")?;
    let label_start = out[..open].rfind('[')?;
    if out[label_start..open].contains(']') {
        return None;
    }
    let label = out[label_start + 1..open].to_string();
    let close = tail.find(')')?;
    out.truncate(label_start);
    out.push_str(&label);
    Some(&tail[close + 1..])
}

// One spreadsheet the user can arm the agent to. `modified_at` disambiguates same-named sheets (the
// app itself can create duplicates), shown as a human date in the arm list.
#[derive(Debug, Clone, PartialEq)]
pub struct Spreadsheet {
    pub id: String,
    pub name: String,
    pub modified_at: String,
}

/// Arm-time: list the user's spreadsheets through the gated agent path so they can pick the one to
/// bind. Calls the read-only `list` tool directly (no LLM) so the ids are exact, not narrated.
pub async fn arm_list_spreadsheets(
    db: &Database,
) -> Result<Vec<Spreadsheet>, String> {
    ensure_agent_installed(db).await?;
    let built = load_built(db, FIXTURE_AGENT_ID)?;
    let backend = BackendClient::from_db(db)
        .ok_or("no backend configured".to_string())?;
    let reg =
        McpRegistry::connect_agent(db, &backend, &built.slug, &built.agent_id)
            .await
            .map_err(|e| format!("mcp connect: {e}"))?;
    let result = reg
        .peer()
        .call_tool(CallToolRequestParams::new(LIST_TOOL))
        .await
        .map_err(|e| format!("list spreadsheets: {e}"))?;
    Ok(parse_spreadsheets(&result_json(&result)))
}

// The bound field the agent pins: a JSON array of armed spreadsheet ids. The gate matches a call when
// its spreadsheet_id is a member, so the agent may act on any armed sheet (RFC 0011).
const BOUND_FIELD: &str = "spreadsheet_id";
// The binding v2 tab pin, matched by the gate on writes and filtered from responses on reads.
const SHEET_FIELD: &str = "sheet";
// Display names live OUTSIDE bound_resource (a name key there would make the gate require it in every
// call's args and deny everything). A JSON map id->name keeps the bound minimal and the UI human.
const ARMED_NAMES_KEY: &str = "armed_sheet_names";

fn read_names(db: &Database) -> serde_json::Map<String, serde_json::Value> {
    db.get_setting(ARMED_NAMES_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_names(
    db: &Database,
    names: &serde_json::Map<String, serde_json::Value>,
) {
    let _ = db.set_setting(
        ARMED_NAMES_KEY,
        &serde_json::Value::Object(names.clone()).to_string(),
    );
}

// One armed binding entry: a spreadsheet, optionally pinned to one tab. Several entries may
// share a spreadsheet (one per pinned tab).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingEntry {
    pub spreadsheet_id: String,
    pub sheet: Option<String>,
}

impl BindingEntry {
    fn whole(id: &str) -> Self {
        Self {
            spreadsheet_id: id.to_string(),
            sheet: None,
        }
    }
}

/// THE single reader for every binding shape ever written: v2 array of entries, v1 object
/// (`{"spreadsheet_id": [ids]}` or a single id), bare legacy scalar. Anything else reads as
/// unarmed - the gate independently fails closed on an unparseable bound.
pub fn parse_binding(v: &serde_json::Value) -> Vec<BindingEntry> {
    match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|e| {
                let id = e.get(BOUND_FIELD)?.as_str()?;
                Some(BindingEntry {
                    spreadsheet_id: id.to_string(),
                    sheet: e
                        .get(SHEET_FIELD)
                        .and_then(|s| s.as_str())
                        .map(String::from),
                })
            })
            .collect(),
        serde_json::Value::Object(_) => match v.get(BOUND_FIELD) {
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|e| e.as_str())
                .map(BindingEntry::whole)
                .collect(),
            Some(serde_json::Value::String(s)) => vec![BindingEntry::whole(s)],
            _ => Vec::new(),
        },
        serde_json::Value::String(s) => vec![BindingEntry::whole(s)],
        _ => Vec::new(),
    }
}

/// The armed entries of the installed agent, through the single reader.
pub fn binding_entries(db: &Database) -> Vec<BindingEntry> {
    db.get_agent_binding(FIXTURE_AGENT_ID)
        .map(|v| parse_binding(&v))
        .unwrap_or_default()
}

// The armed ids, deduped (an id pinned on two tabs is still one armed spreadsheet).
fn bound_ids(db: &Database) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for e in binding_entries(db) {
        if !ids.contains(&e.spreadsheet_id) {
            ids.push(e.spreadsheet_id);
        }
    }
    ids
}

// The gate-format value of a set of entries. The v1 object survives a device downgrade or a
// backup restored on old code, so it stays the wire while no tab is pinned; v2 is written ONLY
// when a `sheet` pin exists (the residual re-open window on old code is confined to tab-pinned
// bindings). None when nothing is armed.
fn binding_value(entries: &[BindingEntry]) -> Option<serde_json::Value> {
    if entries.is_empty() {
        None
    } else if entries.iter().all(|e| e.sheet.is_none()) {
        let ids: Vec<&str> =
            entries.iter().map(|e| e.spreadsheet_id.as_str()).collect();
        Some(serde_json::json!({ BOUND_FIELD: ids }))
    } else {
        Some(serde_json::Value::Array(
            entries
                .iter()
                .map(|e| match &e.sheet {
                    Some(tab) => serde_json::json!({
                        BOUND_FIELD: e.spreadsheet_id, SHEET_FIELD: tab
                    }),
                    None => {
                        serde_json::json!({ BOUND_FIELD: e.spreadsheet_id })
                    }
                })
                .collect(),
        ))
    }
}

/// Persist the armed entries (see `binding_value` for the v1-unless-pinned wire rule). Empty
/// clears to the manifest placeholder, so off-bound is refused until the user re-arms.
pub fn write_binding(
    db: &Database,
    entries: &[BindingEntry],
) -> Result<(), String> {
    match binding_value(entries) {
        None => db.set_agent_binding(FIXTURE_AGENT_ID, None),
        Some(v) => db.set_agent_binding(FIXTURE_AGENT_ID, Some(&v.to_string())),
    }
}

/// The device bindings per connector SLUG, in gate format, for the chat surface. Today the
/// only armable binding belongs to the installed CRM agent; its connector resolves by type,
/// exactly as the chain path resolves it - one source of truth for both surfaces.
pub fn armed_bounds(
    db: &Database,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut out = std::collections::BTreeMap::new();
    let Some(bound) = binding_value(&binding_entries(db)) else {
        return out;
    };
    if let Some((slug, _)) =
        crate::application::connector_pins::resolve_for_type(
            db,
            "tabular_store",
        )
    {
        out.insert(slug, bound);
    }
    out
}

/// Arm the installed agent to one more spreadsheet: add its id to the bound set and record its display
/// name. Idempotent on the id. The builder merges the set over the manifest on the next run, so the
/// agent may read/write any armed sheet and no other. Captures the sheet's header schema per tab as
/// part of arming: headers the gate can later validate header-keyed writes against. A schema that is
/// structurally unusable (duplicate or empty header cells) refuses the arm with the exact problem; a
/// capture that merely could not run (offline, backend down, tool not reachable yet) arms anyway -
/// the schema stays absent and a later re-sync heals it.
pub async fn bind_spreadsheet(
    db: &Database,
    spreadsheet_id: &str,
    name: &str,
) -> Result<(), String> {
    require_installed(db)?;
    match capture_sheet_schema(db, spreadsheet_id).await {
        Ok(()) | Err(CaptureError::Unavailable(_)) => {}
        Err(CaptureError::Invalid(msg)) => return Err(msg),
    }
    let mut entries = binding_entries(db);
    if !entries.iter().any(|e| e.spreadsheet_id == spreadsheet_id) {
        entries.push(BindingEntry::whole(spreadsheet_id));
    }
    write_binding(db, &entries)?;
    let mut names = read_names(db);
    names.insert(spreadsheet_id.to_string(), name.into());
    write_names(db, &names);
    Ok(())
}

/// Pin one armed spreadsheet to a set of tabs. Empty = whole workbook (the pin clears).
/// Replaces that spreadsheet's entries; other armed spreadsheets are untouched.
pub fn set_sheet_pins(
    db: &Database,
    spreadsheet_id: &str,
    tabs: &[String],
) -> Result<(), String> {
    let mut entries: Vec<BindingEntry> = binding_entries(db)
        .into_iter()
        .filter(|e| e.spreadsheet_id != spreadsheet_id)
        .collect();
    if tabs.is_empty() {
        entries.push(BindingEntry::whole(spreadsheet_id));
    } else {
        for tab in tabs {
            entries.push(BindingEntry {
                spreadsheet_id: spreadsheet_id.to_string(),
                sheet: Some(tab.clone()),
            });
        }
    }
    write_binding(db, &entries)
}

/// Disarm one spreadsheet by id. When the last one is removed the binding clears to the manifest
/// placeholder, so off-bound reads/writes are refused again until the user re-arms.
pub fn unbind_spreadsheet(
    db: &Database,
    spreadsheet_id: &str,
) -> Result<(), String> {
    let mut entries = binding_entries(db);
    entries.retain(|e| e.spreadsheet_id != spreadsheet_id);
    write_binding(db, &entries)?;
    let mut names = read_names(db);
    names.remove(spreadsheet_id);
    write_names(db, &names);
    let mut schema = read_schema(db);
    if schema.remove(spreadsheet_id).is_some() {
        write_schema(db, &schema);
    }
    Ok(())
}

// --- armed sheet schema: the per-tab header rows captured when a spreadsheet is armed ---

// JSON map in settings: {spreadsheet_id: {sheet_name: {"headers": [...], "captured_at": iso}}}.
// Device-local by design - the schema belongs to the armed sheet, not to the signed manifest.
const ARMED_SCHEMA_KEY: &str = "armed_sheet_schema";
// Synthetic proxy tool: row 1 of a tab as clean JSON. The klavis read tools serialize cell data
// as a Python repr string, unusable as a schema source.
const GET_HEADERS_TOOL: &str = "google_sheets_get_headers";

enum CaptureError {
    // The sheet itself cannot serve header-keyed writes (duplicate/empty header cells).
    Invalid(String),
    // The capture could not run (no backend, MCP unreachable, tool denied). Not the sheet's fault.
    Unavailable(String),
}

fn read_schema(db: &Database) -> serde_json::Map<String, serde_json::Value> {
    db.get_setting(ARMED_SCHEMA_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_schema(
    db: &Database,
    schema: &serde_json::Map<String, serde_json::Value>,
) {
    let _ = db.set_setting(
        ARMED_SCHEMA_KEY,
        &serde_json::Value::Object(schema.clone()).to_string(),
    );
}

/// The whole armed schema map, for a run's contract snapshot (the hook validates and re-syncs
/// against its own copy - it has no Database handle).
pub fn armed_schema_map(
    db: &Database,
) -> serde_json::Map<String, serde_json::Value> {
    read_schema(db)
}

/// Persist a schema map a run refreshed (the hook's re-syncs write back through here at run end).
pub fn store_schema_map(
    db: &Database,
    schema: &serde_json::Map<String, serde_json::Value>,
) {
    write_schema(db, schema);
}

/// The captured headers for one (spreadsheet, tab), if a capture has run. None = never captured
/// (schema unknown); Some(empty) = captured and the tab has no header row yet.
pub fn armed_schema_for(
    db: &Database,
    spreadsheet_id: &str,
    sheet: &str,
) -> Option<Vec<String>> {
    read_schema(db)
        .get(spreadsheet_id)?
        .get(sheet)?
        .get("headers")?
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
}

/// Header row validation: every cell must be non-empty and unique, else the tab cannot address
/// columns by name. An empty row is fine (sheet not set up yet - writes stay refused until it is).
pub fn validate_headers(raw: &[String]) -> Result<Vec<String>, String> {
    let mut seen: Vec<&str> = Vec::with_capacity(raw.len());
    for h in raw {
        let h = h.trim();
        if h.is_empty() {
            return Err(
                "header row has an empty cell; every column needs a name"
                    .into(),
            );
        }
        if seen.contains(&h) {
            return Err(format!(
                "duplicate header \"{h}\"; column names must be unique"
            ));
        }
        seen.push(h);
    }
    Ok(raw.iter().map(|h| h.trim().to_string()).collect())
}

// Tab titles from a `list_sheets` response: the `sheets` array, elements either plain strings or
// objects carrying a `title`. `pub` so the parse is tested against the real connector shape.
pub fn sheet_titles_from_meta(v: &serde_json::Value) -> Vec<String> {
    v.get("sheets")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    e.as_str().map(String::from).or_else(|| {
                        e.get("title")
                            .and_then(|t| t.as_str())
                            .map(String::from)
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

// Read and store the header schema for every tab of one spreadsheet. Validation failure on any
// tab refuses the whole capture (Invalid); a transport/tool failure is Unavailable.
async fn capture_sheet_schema(
    db: &Database,
    spreadsheet_id: &str,
) -> Result<(), CaptureError> {
    let unavailable = |m: String| CaptureError::Unavailable(m);
    let built = load_built(db, FIXTURE_AGENT_ID).map_err(unavailable)?;
    let backend = BackendClient::from_db(db)
        .ok_or_else(|| unavailable("no backend configured".into()))?;
    let reg =
        McpRegistry::connect_agent(db, &backend, &built.slug, &built.agent_id)
            .await
            .map_err(|e| unavailable(format!("mcp connect: {e}")))?;

    let mut meta_args = serde_json::Map::new();
    meta_args.insert("spreadsheet_id".to_string(), spreadsheet_id.into());
    let meta = reg
        .peer()
        .call_tool(
            CallToolRequestParams::new(SHEET_META_TOOL)
                .with_arguments(meta_args),
        )
        .await
        .map_err(|e| unavailable(format!("list sheets: {e}")))?;
    let titles = sheet_titles_from_meta(&result_json(&meta));
    if titles.is_empty() {
        return Err(unavailable("no tabs found in sheet metadata".into()));
    }

    let mut tabs = serde_json::Map::new();
    for title in titles {
        let raw = fetch_tab_headers(&reg.peer(), spreadsheet_id, &title)
            .await
            .map_err(unavailable)?;
        let headers = validate_headers(&raw).map_err(|e| {
            CaptureError::Invalid(format!("tab \"{title}\": {e}"))
        })?;
        tabs.insert(title, tab_schema_entry(&headers));
    }

    let mut schema = read_schema(db);
    schema.insert(spreadsheet_id.to_string(), serde_json::Value::Object(tabs));
    write_schema(db, &schema);
    Ok(())
}

/// One tab's schema entry as stored in the armed schema map.
pub fn tab_schema_entry(headers: &[String]) -> serde_json::Value {
    serde_json::json!({
        "headers": headers,
        "captured_at": chrono::Utc::now().to_rfc3339(),
    })
}

/// Row-1 headers of one tab through an already-connected MCP peer. Used by arm-time capture and
/// by the gate's re-sync (the hook holds a peer, never a Database).
pub async fn fetch_tab_headers(
    peer: &rmcp::service::ServerSink,
    spreadsheet_id: &str,
    sheet: &str,
) -> Result<Vec<String>, String> {
    let mut args = serde_json::Map::new();
    args.insert("spreadsheet_id".to_string(), spreadsheet_id.into());
    args.insert("sheet".to_string(), sheet.into());
    let result = peer
        .call_tool(
            CallToolRequestParams::new(GET_HEADERS_TOOL).with_arguments(args),
        )
        .await
        .map_err(|e| format!("get headers ({sheet}): {e}"))?;
    result_json(&result)
        .get("headers")
        .and_then(|h| h.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .ok_or_else(|| format!("get headers ({sheet}): unexpected shape"))
}

/// Re-capture the schema of one armed spreadsheet on demand (the gate's mismatch path, or a manual
/// refresh). Unlike arm-time capture, every failure surfaces: a requested re-sync must be honest.
pub async fn resync_sheet_schema(
    db: &Database,
    spreadsheet_id: &str,
) -> Result<(), String> {
    capture_sheet_schema(db, spreadsheet_id)
        .await
        .map_err(|e| match e {
            CaptureError::Invalid(m) | CaptureError::Unavailable(m) => m,
        })
}

/// Extract the spreadsheet id from a pasted Google Sheets URL: the `<id>` in the
/// `/spreadsheets/d/<id>/` segment (the inverse of the UI's `sheet_url`). None when there is no such
/// segment, so a stray paste is rejected instead of armed to garbage.
pub fn spreadsheet_id_from_url(url: &str) -> Option<String> {
    let after = url.split("/d/").nth(1)?;
    let id: String = after
        .chars()
        .take_while(|c| {
            !c.is_whitespace() && *c != '/' && *c != '?' && *c != '#'
        })
        .collect();
    (!id.is_empty()).then_some(id)
}

/// Arm the agent by pasting the sheet's URL instead of picking from the list. Resolves the real title
/// by id (works whether or not it shows up in the Drive listing); falls back to the id if unresolved.
pub async fn bind_spreadsheet_from_url(
    db: &Database,
    url: &str,
) -> Result<(), String> {
    ensure_agent_installed(db).await?;
    let id = spreadsheet_id_from_url(url)
        .ok_or("not a Google Sheets URL".to_string())?;
    let name = fetch_spreadsheet_name(db, &id)
        .await
        .unwrap_or_else(|| id.clone());
    bind_spreadsheet(db, &id, &name).await
}

// Resolve one spreadsheet's title by id via `list_sheets` - the Sheets metadata read
// (fields=properties/title, NO cell data), so it is small and works for any accessible sheet, owned or
// shared. `get_spreadsheet` is the wrong tool here: it sets includeGridData and drags every cell just to
// read a title. The title is the response's top-level `title`. Best-effort: None on any failure.
const SHEET_META_TOOL: &str = "google_sheets_list_sheets";

pub async fn fetch_spreadsheet_name(db: &Database, id: &str) -> Option<String> {
    let meta = fetch_sheet_meta(db, id).await.ok()?;
    title_from_meta(&meta)
}

/// The tab titles of one spreadsheet BEFORE it is armed, for the per-tab pin checkboxes.
/// Same metadata fetch the arm-time schema capture uses (titles only, no cell data).
pub async fn list_spreadsheet_tabs(
    db: &Database,
    spreadsheet_id: &str,
) -> Result<Vec<String>, String> {
    let meta = fetch_sheet_meta(db, spreadsheet_id).await?;
    let titles = sheet_titles_from_meta(&meta);
    if titles.is_empty() {
        Err("no tabs found in sheet metadata".to_string())
    } else {
        Ok(titles)
    }
}

// One spreadsheet's metadata (title + tabs) through the agent-scoped MCP route.
async fn fetch_sheet_meta(
    db: &Database,
    spreadsheet_id: &str,
) -> Result<serde_json::Value, String> {
    let built = load_built(db, FIXTURE_AGENT_ID)?;
    let backend = BackendClient::from_db(db)
        .ok_or("no backend configured".to_string())?;
    let reg =
        McpRegistry::connect_agent(db, &backend, &built.slug, &built.agent_id)
            .await
            .map_err(|e| format!("mcp connect: {e}"))?;
    let mut args = serde_json::Map::new();
    args.insert("spreadsheet_id".to_string(), spreadsheet_id.into());
    let result = reg
        .peer()
        .call_tool(
            CallToolRequestParams::new(SHEET_META_TOOL).with_arguments(args),
        )
        .await
        .map_err(|e| format!("list sheets: {e}"))?;
    Ok(result_json(&result))
}

// The spreadsheet title from a `list_sheets` response: the top-level `title` (the connector returns
// {spreadsheetId, url, title, sheets:[...]}). `pub` so the parse is tested against the real shape.
pub fn title_from_meta(v: &serde_json::Value) -> Option<String> {
    v.get("title")
        .and_then(|t| t.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn is_named(
    names: &serde_json::Map<String, serde_json::Value>,
    id: &str,
) -> bool {
    names
        .get(id)
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.is_empty() && s != id)
}

/// Heal bindings whose stored name is still their id (a legacy single-bind, or a URL bind made before
/// the title was resolvable) by reading each title by id now. Returns the refreshed pairs.
pub async fn resolve_missing_names(db: &Database) -> Vec<(String, String)> {
    let ids = bound_ids(db);
    let names_snapshot = read_names(db);
    let missing: Vec<String> = ids
        .iter()
        .filter(|id| !is_named(&names_snapshot, id))
        .cloned()
        .collect();
    if missing.is_empty() {
        return current_bindings(db);
    }
    let mut names = names_snapshot;
    let mut changed = false;
    for id in missing {
        if let Some(name) = fetch_spreadsheet_name(db, &id).await {
            names.insert(id, name.into());
            changed = true;
        }
    }
    if changed {
        write_names(db, &names);
    }
    current_bindings(db)
}

/// The (id, display name) pairs the agent is currently armed to, for the arm screen. A name falls back
/// to the id when it was not recorded. Empty when unbound.
pub fn current_bindings(db: &Database) -> Vec<(String, String)> {
    let names = read_names(db);
    bound_ids(db)
        .into_iter()
        .map(|id| {
            let name = names
                .get(&id)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from)
                .unwrap_or_else(|| id.clone());
            (id, name)
        })
        .collect()
}

/// Flatten a tool result to a single JSON value: prefer the structured payload, else parse the joined
/// text content. The Sheets connector serializes its list with Python `str()`, not `json.dumps`, so
/// the text is a Python dict literal - try strict JSON first, then normalize the Python repr to JSON.
/// `pub` because the hook's response filter flattens the results it executes through the same path.
pub fn result_json(result: &rmcp::model::CallToolResult) -> serde_json::Value {
    if let Some(structured) = &result.structured_content {
        return structured.clone();
    }
    let text = result
        .content
        .iter()
        .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    serde_json::from_str(&text)
        .or_else(|_| serde_json::from_str(&python_literal_to_json(&text)))
        .unwrap_or(serde_json::Value::Null)
}

/// Normalize a Python dict-literal string (single quotes, `None`/`True`/`False`) to JSON. Walks the
/// source tracking string boundaries, so an apostrophe inside a value (`"L'Oreal"`) is preserved
/// instead of breaking the quotes. Valid JSON passes through unchanged, so it is safe to apply when
/// the strict parse already failed. `pub` so the normalization is tested without a live backend.
pub fn python_literal_to_json(src: &str) -> String {
    let mut out = String::with_capacity(src.len() + 8);
    let mut chars = src.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '\'' | '"' => {
                let delim = c;
                chars.next();
                let mut value = String::new();
                while let Some(ch) = chars.next() {
                    if ch == '\\' {
                        // Decode the escape to its actual char; JSON re-encoding happens below.
                        match chars.next() {
                            Some('n') => value.push('\n'),
                            Some('t') => value.push('\t'),
                            Some('r') => value.push('\r'),
                            Some(other) => value.push(other),
                            None => break,
                        }
                    } else if ch == delim {
                        break;
                    } else {
                        value.push(ch);
                    }
                }
                out.push_str(
                    &serde_json::to_string(&value)
                        .unwrap_or_else(|_| "\"\"".into()),
                );
            }
            'A'..='Z' | 'a'..='z' | '_' => {
                let mut word = String::new();
                while let Some(&w) = chars.peek() {
                    if w.is_ascii_alphanumeric() || w == '_' {
                        word.push(w);
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push_str(match word.as_str() {
                    "None" => "null",
                    "True" => "true",
                    "False" => "false",
                    other => other,
                });
            }
            _ => {
                out.push(c);
                chars.next();
            }
        }
    }
    out
}

// Pull (id, name) pairs out of whatever shape the connector returns: the first array of objects
// anywhere in the payload, keyed loosely since connectors differ. An object with no id is skipped.
// `pub` so the parser is tested against representative payloads without a live backend.
pub fn parse_spreadsheets(v: &serde_json::Value) -> Vec<Spreadsheet> {
    let Some(arr) = first_object_array(v) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|o| {
            let id =
                pick(o, &["id", "spreadsheet_id", "spreadsheetId", "fileId"])?;
            let name = pick(o, &["name", "title", "spreadsheet_name"])
                .unwrap_or_else(|| id.clone());
            let modified_at =
                pick(o, &["modifiedAt", "modified_at", "modifiedTime"])
                    .unwrap_or_default();
            Some(Spreadsheet {
                id,
                name,
                modified_at,
            })
        })
        .collect()
}

fn pick(o: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| o.get(k).and_then(|v| v.as_str()))
        .map(str::to_string)
}

fn first_object_array(
    v: &serde_json::Value,
) -> Option<&Vec<serde_json::Value>> {
    match v {
        serde_json::Value::Array(arr) if arr.iter().any(|e| e.is_object()) => {
            Some(arr)
        }
        serde_json::Value::Object(map) => {
            map.values().find_map(first_object_array)
        }
        serde_json::Value::Array(arr) => {
            arr.iter().find_map(first_object_array)
        }
        _ => None,
    }
}

// Load the pinned row (placed by `ensure_installed`), re-check its integrity against the pinned
// digest, and build the runnable agent from its manifest. Sync: the network install happens upstream.
fn load_built(db: &Database, agent_id: &str) -> Result<BuiltAgent, String> {
    let installed = db
        .get_installed_agent(agent_id)
        .ok_or("agent not installed")?;

    let recomputed = digest_of_stored(&installed.manifest_json)
        .map_err(|e| e.to_string())?;
    if recomputed != installed.content_digest {
        return Err(format!(
            "pinned digest mismatch for `{agent_id}`: stored manifest no longer hashes to its pin"
        ));
    }

    let mut manifest =
        parse_manifest(&installed.manifest_json).map_err(|e| e.to_string())?;
    // Overlay the arm-time binding so validation and the gate target the sheet the user picked, not
    // the manifest placeholder. Unbound -> the placeholder stands and off-bound writes stay refused.
    if let Some(binding) = db.get_agent_binding(agent_id) {
        merge_bound(&mut manifest.governance.bound_resource, binding);
    }
    // Data-driven connector resolution (RFC 0016): the pinned manifest, lowest slug of the type.
    // No pin (never fetched, or revoked) -> the agent is disarmed with a clear message.
    let ctype =
        crate::application::agent_builder::required_connector_type(&manifest)?;
    let (slug, conn_json) = crate::application::connector_pins::resolve_for_type(db, ctype)
        .ok_or_else(|| {
            format!("no connector pinned for type `{ctype}` (revoked or not yet installed)")
        })?;
    build_agent(&manifest, &slug, &conn_json)
}

// Arm-time install of the CRM module's agent. The generic install lives in
// `agent_directory::ensure_installed`; this keeps the sync/arm paths pinned to their agent.
async fn ensure_agent_installed(db: &Database) -> Result<(), String> {
    crate::application::agent_directory::ensure_installed(db, FIXTURE_AGENT_ID)
        .await
}

// Sync guard for the list-pick arm path, which always follows `arm_list_spreadsheets` (where the
// network install already ran). Bind targets the pinned row's `bound_json`, so a missing row would
// silently drop the binding - refuse it with a clear message instead.
fn require_installed(db: &Database) -> Result<(), String> {
    if db.get_installed_agent(FIXTURE_AGENT_ID).is_some() {
        Ok(())
    } else {
        Err(format!(
            "agent `{FIXTURE_AGENT_ID}` not installed - open Connections to install it"
        ))
    }
}

// The chat bubble is the ANSWER, not the log: final_text first, then one discreet footer line
// naming the tools that ran (deduped, no args, no payloads). A blocked run (empty final text)
// surfaces the last step's outcome - the human-readable block reason - instead. The full
// ground-truth trace goes to the device log only, where a debugging eye can find it.
pub fn format_outcome(outcome: &ChainOutcome) -> String {
    for step in &outcome.trace {
        eprintln!("[chain:{}] {}", step.state, step.outcome);
        for line in &step.tools {
            eprintln!("[chain:{}]   - {line}", step.state);
        }
    }

    // A run that broke early (guard block, no servable tool) never reached the terminal
    // synthesis: its final_text is the raw accumulated transcript. Surface the last step's
    // human-readable reason instead of that noise.
    let broke_early = outcome
        .trace
        .last()
        .map(|s| {
            s.outcome.starts_with("blocked")
                || s.outcome.starts_with("unavailable")
        })
        .unwrap_or(false);
    let answer = outcome.final_text.trim();
    let body = if broke_early || answer.is_empty() {
        outcome
            .trace
            .last()
            .map(|s| s.outcome.trim().to_string())
            .unwrap_or_default()
    } else {
        answer.to_string()
    };

    let tools = called_tools(outcome);
    if tools.is_empty() {
        body
    } else {
        format!("{body}\n\n---\n_🛠 {}_", tools.join(" · "))
    }
}

// Tool names in call order, deduped, extracted from the ground-truth log lines
// ("called <tool> <args> -> <decision>"). Language-free by design: names only.
fn called_tools(outcome: &ChainOutcome) -> Vec<String> {
    let mut seen = Vec::new();
    for step in &outcome.trace {
        for line in &step.tools {
            let mut words = line.split_whitespace();
            if words.next() == Some("called") {
                if let Some(name) = words.next() {
                    if !seen.iter().any(|s| s == name) {
                        seen.push(name.to_string());
                    }
                }
            }
        }
    }
    seen
}
