---
artifact: docs/rfcs/0008-mcp-connectors-oauth-broker-backend/RFC.md
artifact_kind: rfc
gate: Accepted
scope: [T00a, T00b, T02b]
engine_tier: solo
stepsCompleted: [0, 1, 2, 3]
final_status: in_progress
spike_root: ~/code/flowflow-spikes
---

# ship trace ledger - RFC 0008 spikes

Survives context reset. One row per task; stop-after-each per Mirko.

| Task | Status | Spike crate | Result |
|------|--------|-------------|--------|
| T00a | PASS (host) | ~/code/flowflow-spikes/t00a-rmcp | builds; connects DeepWiki; lists 3 tools; .rmcp_tools() OK; rmcp single v1.7.0 |
| T00b | vérif1+2 PASS on device; vérif3 (auth_header) pending | in-app debug btn | link OK; iPhone connected DeepWiki MCP, listed 3 tools on device |
| T02b | SUPERSEDED by pivot | - | GitHub dropped; replaced by Klavis open-strata + Google Sheets CRM (issue #62 repurposed) |

## T00a RESULT - PASS (host, 2026-06-19)
- `.rmcp_tools(tools, client.peer().to_owned())` is the builder method. OQ#1 RESOLVED. Finding 11 verified on host.
- `cargo tree -i rmcp` -> ONE version: **rmcp v1.7.0**, shared by rig-core 0.36 + direct dep. Finding 17 = non-issue.
- Connected to public Streamable-HTTP MCP (DeepWiki), listed 3 tools. Agent built (OPENAI_API_KEY was present).
- rmcp 1.7 API drift vs the rig 0.23-era doc (RECORD for T07/T08 app wiring):
  - `ClientInfo` / `Implementation` are `#[non_exhaustive]` -> use `ClientInfo::new(caps, Implementation::new(name, ver))`, not struct literals.
  - `openai::Client::from_env()` returns `Result` -> needs `?`.
  - Trait imports required in scope: `rig::client::ProviderClient` (from_env), `rig::client::CompletionClient` (.agent()).
  - Cargo features that work: rig-core `["rmcp"]`; rmcp `["client","macros","transport-streamable-http-client-reqwest"]`.

## API resolved (research, pre-code)
- rig 0.36 ships `rmcp` feature. Agent builder exposes `.rmcp_tools(tools, peer)` (static snapshot)
  AND `.tool_server_handle(handle)` + `McpClientHandler::connect(transport)` (dynamic, refreshes on list_changed).
- Low-level adapter: `McpTool::from_mcp_server(tool, server_sink)`.
- Client: `rmcp::transport::StreamableHttpClientTransport::from_uri(url)` -> `ClientInfo.serve(transport)`
  (needs `use rmcp::ServiceExt;`) -> `client.list_tools(Default::default()).await?.tools` -> `client.peer()`.
- rmcp SDK latest 1.7.0; rig 0.36 pins some 1.x -> T00a confirms unification via `cargo tree -i rmcp`.

## T00b harness (temporary, in flowflow - REVERT after validation)
- `Cargo.toml`: rig-core features += "rmcp"; added `rmcp = {1, default-features=false, features=[client,macros,transport-streamable-http-client-reqwest]}`.
- `src/services/mcp_spike.rs` (new): `run_mcp_spike(url) -> Result<String,String>`, connects + lists tools. `DEFAULT_MCP_URL = DeepWiki`.
- `src/services/mod.rs`: `pub mod mcp_spike;`.
- `src/ui/settings/privacy.rs`: "Run MCP spike" button + on-screen result (Settings -> Privacy, bottom).
- Validation = make all -> iPhone, Settings > Privacy > "Run MCP spike", read line. Watch iOS link (reqwest tls feature unification is the risk).
- REVERT list when done: drop mcp_spike.rs, the mod line, the privacy.rs block, and the Cargo rmcp dep (keep only if T07 wants it).

## T00a log
- Spike uses the simple `.rmcp_tools(..)` path (doc-blessed). Endpoint via argv[1]/MCP_URL, default DeepWiki public MCP.
- Agent build guarded behind OPENAI_API_KEY (compile still proves the builder API).
