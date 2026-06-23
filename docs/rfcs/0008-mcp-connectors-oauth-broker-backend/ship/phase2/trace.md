---
artifact: docs/rfcs/0008-mcp-connectors-oauth-broker-backend/RFC.md
artifact_kind: rfc
gate: Accepted
phase: 2 (device, flowflow repo)
scope: [T05, T06, T07, T08, T09, T13]
engine_tier: solo
backend_repo: /Users/mirkobozzetto/code/marketplace-flowflow
contract: ship/phase2/contract.md
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: shipped
updated: 2026-06-19
---

# Trace ledger: RFC 0008 Phase 2 (device MCP client + OAuth + Connections)

Single source of truth. Survives context reset. One row per task; STOP after each per Mirko's methodology.
Backend API mapped from marketplace-flowflow: single fixed proxy `POST /v1/mcp`; auth challenge/verify/refresh;
sign the nonce STRING as UTF-8 bytes (verify_strict); base64 standard+padding on the wire.

## DAG (topological)

T05, T06 (disjoint) -> T07 -> T08 ; T05+T06 -> T09 ; T08+T09 -> T13
Critical path: T05 -> T07 -> T08 -> T13 (T09 parallel into T13). Cargo.toml shared by T05/T06 -> serialize that edit.

## Tasks

| Task | Status | Files | Notes |
|------|--------|-------|-------|
| T05 | DONE (code+unit) | Cargo.toml(+ed25519-dalek), services/backend/mod.rs(new), db/settings_repo.rs, services/mod.rs | BackendClient: ensure_identity (Ed25519, base64-std, persisted once), session() = hydrate/refresh/re-auth, mutex-serialized (no double-refresh). Signs nonce STRING bytes (matches verify_strict). 3 sensitive keys + 2 device-local added to backup-exclusion. 2 unit tests green, clippy clean. Live handshake rides on T07 device test. |
| T06 | DONE (code+iOS link) | Cargo.toml(+objc2-authentication-services), platform/ios/oauth.rs(new), platform/ios/mod.rs, platform/mod.rs | perform_oauth: ASWebAuthenticationSession + AnchorProvider (define_class MainThreadOnly; anchor selector registered via plain impl since binding gates it to macOS; returns +0 *mut NSObject = key window). RcBlock(NSURL,NSError) completion + poll (mirrors reminders.rs). callbackURLScheme in-session capture -> NO Dioxus.toml/[deep_links] needed (finding 33 resolved). Non-iOS stub returns Err (desktop gated, finding 29). aarch64-apple-ios links; clippy clean (mine). Runtime device test (opens sheet/returns URL/no-crash) rides on T09 connect button. |
| T07 | DONE (code+unit) | services/mcp/mod.rs(new), services/mod.rs | McpRegistry::connect -> from_config(with_uri({backend}/v1/mcp).auth_header(session)); serve+list_tools; holds RunningService alive (peer validity). rmcp 1.7 API: StreamableHttpClientTransportConfig.auth_header(token) sends Bearer (finding 12). 1 unit test (auth_header wiring), clippy clean. iOS build deferred to T08 (rmcp_tools is the new iOS surface). |
| T08 | DONE (code+unit+iOS link) | services/llm.rs, services/constants.rs, services/mod.rs, ui/settings/privacy.rs; TRASHED mcp_spike.rs | prompt_with_agent: connect_mcp() before the match, conditional builder.rmcp_tools(tools,peer) per arm (typestate: WithBuilderTools, rmcp_tools returns Self). Graceful fallback (connect fail -> notes-only). Preamble: connector-tools line in both RAG prompts. Spike fully reverted (file trashed, mod line + privacy debug block gone; rmcp Cargo deps KEPT, T07 uses them). Finding-32 unit test (from_db None when unconfigured). clippy clean; aarch64-apple-ios links (579 crates, 3m13s) - de-risks rig[rmcp]+rmcp_tools on device. |
| T09 | DONE (code+unit+iOS link) | Cargo.toml(+url), backend/mod.rs(connector methods), ui/state.rs, ui/settings/connections.rs(new), ui/settings/mod.rs, i18n en+fr.ftl | SCOPE DELTA (Mirko-approved): SettingsSection::Connections, NOT top-level View - design-consistent entry point. BackendClient: list_connectors/authorize/complete_callback/disconnect + authed() helper (Bearer + 401-retry-with-fresh-session). connect_flow: authorize -> perform_oauth(scheme=flowflow) -> parse_callback (url crate, %-decoded) -> complete_callback. Desktop: connect gated off ("iOS only", finding 29). base_url input writes setting + reloads. 10 i18n keys EN+FR. 2 parse_callback tests. clippy clean, aarch64-apple-ios links. |
| T13 | DONE (code+suite) | ui/chat/actions.rs, i18n en+fr.ftl | chat_error_message(): connector/auth errors (unauthorized/connector/backend/mcp in the message) get an actionable "open Connections" hint (chat-connector-error key EN+FR); else generic chat-error. Connector STATUS reflected by the Connections list (T09, backend flips connected=false on dead refresh). LIMIT: only errors that PROPAGATE surface here; rig may feed a mid-turn tool error to the model, and connect_mcp failure falls back to notes-only silently - Connections screen is the connector-health source of truth. Full host suite 68 pass. |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-02 | scope delta | T13 file = ui/chat/actions.rs (RFC said chat.rs) | code evolved to ui/chat/ module |
| step-02 | scope delta | new Ed25519 key, not the sync x25519 key | sync key is Noise-DH, cannot sign (RFC §6 already notes this) |
| step-03 | engine | solo | project methodology = one step + device validation between tasks; ultracode not requested |

## Open decision (flagged, not blocking the build)

- OAuth callback transport: `flowflow://` custom scheme vs HTTPS universal-link. T06 built scheme-parameterized
  (default `flowflow`); couples to backend GOOGLE_REDIRECT_URI + Google OAuth app registration (ops, Mirko).

## HALT events

- None.
