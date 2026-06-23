---
artifact: docs/rfcs/0008-mcp-connectors-oauth-broker-backend/RFC.md
artifact_kind: rfc
gate: Accepted
phase: 2 (device, flowflow repo)
scope: [T05, T06, T07, T08, T09, T13]
engine_tier: solo
backend_repo: /Users/mirkobozzetto/code/marketplace-flowflow
backend_api: single fixed MCP proxy POST /v1/mcp; auth challenge/verify/refresh; google connector authorize/callback/disconnect
---

# Contract: RFC 0008 Phase 2 (device-side MCP client + OAuth + Connections UI)

Definition of done. The verification bundle validates against THESE rows. A requirement change
gets a NEW row, never a silent rewrite. ship does NOT run the toolchain; Mirko runs the bundle + device.

## Acceptance criteria (one row per §10 CURRENT-plan Accept cell)

| ID | Criterion | Source files (edit scope) | Verify |
|----|-----------|---------------------------|--------|
| C-T05 | New Ed25519 device key persisted + backup-excluded (distinct from the x25519 sync key). `BackendClient` does challenge -> sign nonce STRING bytes -> verify -> caches session + rotates via `/v1/auth/refresh`. | `Cargo.toml` (+ed25519-dalek), `src/services/backend/mod.rs` (new), `src/db/settings_repo.rs` | unit: sign/verify roundtrip, key persisted once; device: obtains a session against the deployed backend |
| C-T06 | `ios/oauth.rs` ASWebAuthenticationSession wrapper: opens the consent sheet, returns the callback URL to Rust, no crash across repeated runs. Parameterized callback scheme (default `flowflow`). | `Cargo.toml` (+objc2-authentication-services, iOS), `src/platform/ios/oauth.rs` (new), `src/platform/ios/mod.rs`, `src/platform/mod.rs`, `Dioxus.toml` | device only: open + return URL + repeat-run no-crash (findings 16,33) |
| C-T07 | `McpRegistry` (rmcp StreamableHttp) points at `{backend}/v1/mcp`, `auth_header = Bearer {session}`; connects, lists tools, the session `auth_header` reaches the device's own SSE leg (finding 12). | `src/services/mcp/mod.rs` (new), `src/services/mod.rs` | device: lists tools through the proxy (needs premium=1 server-side) |
| C-T08 | Non-empty registry -> MCP tools registered on the agent + preamble mentions connectors. EMPTY registry -> exactly the 3 notes tools, pre-change path (finding 32). Spike harness removed. | `src/services/llm.rs`, `src/services/constants.rs`; REMOVE `src/services/mcp_spike.rs` + its `mod` line + the privacy.rs debug block | unit: empty-registry tool-set == 3; build green; harness gone |
| C-T09 | `View::Connections`: lists connectors, connect launches the WebAuth sheet, status flips on connect, desktop shows "iOS only". | `src/ui/state.rs`, `src/ui/connections.rs` (new), `src/ui/mod.rs` | device: connect flow; desktop: gated affordance |
| C-T13 | A chat tool-call 401/timeout/failure -> a chat message + connector status reflects the failure (finding 21). | `src/ui/chat/actions.rs` (RFC said `chat.rs`; code is `chat/`) | device/manual: forced failure shows graceful error |

## Out of scope (never build here)

- Monetization / IAP / paywall / StoreKit (Non-Goal 1; separate RFC).
- MCP marketplace / catalog / multi-connector store (Non-Goal 2).
- Touching the P2P sync layer (Non-Goal 3) - reuse the keypair PATTERN only, new key.
- Server-side LLM inference (Non-Goal 4); user accounts/email (Non-Goal 5); Android (Non-Goal 6); inbound/push (Non-Goal 7).
- Phase 1 backend (shipped in marketplace-flowflow) and Phase 3 T10 E2E (separate scope).
- App Attest/DeviceCheck and identity-recovery (RFC-deferred, documented limitations).

## Known preconditions for LIVE device verification (ops, Mirko - NOT code, NOT in scope)

- Backend deployed on Dokploy over HTTPS; `backend_base_url` set in the app Settings/DB.
- Google OAuth app registered (`drive.file`); `GOOGLE_REDIRECT_URI` matches the app callback scheme (see DECISION below).
- The device row marked `premium=1` server-side (authorize/callback/`/v1/mcp` are premium-gated -> 403 otherwise).

## DECISION needed before T06 live test (does not block the build)

OAuth callback transport: custom scheme `flowflow://...` (RFC flow, app-captured) vs HTTPS universal-link
(backend `.env.example` default). The wrapper is built scheme-parameterized either way; only the backend
`GOOGLE_REDIRECT_URI` + the Google OAuth app registration differ. Default assumed: `flowflow`.

## Edit-scope union (authorized files)

Cargo.toml; Dioxus.toml; src/services/{backend/mod.rs(new), mcp/mod.rs(new), mod.rs, llm.rs, constants.rs};
src/db/settings_repo.rs; src/platform/{mod.rs, ios/mod.rs, ios/oauth.rs(new)}; src/ui/{state.rs,
connections.rs(new), mod.rs, chat/actions.rs}; REMOVE src/services/mcp_spike.rs + src/ui/settings/privacy.rs debug block.
