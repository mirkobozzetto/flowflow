# Verification bundle: RFC 0008 Phase 2 (device)

Scope shipped: T05, T06, T07, T08, T09, T13. Engine: solo. Contract: `ship/phase2/contract.md`.

## Toolchain (ship already ran these; re-run to confirm)

```bash
cargo test --lib                                                              # 68 pass (62 baseline + 6 new)
cargo clippy --lib                                                            # clean
IPHONEOS_DEPLOYMENT_TARGET=16.0 cargo build --lib --features mobile --target aarch64-apple-ios   # links
cargo clippy --lib --features mobile --target aarch64-apple-ios               # only pre-existing iOS warnings
```

New unit tests: `backend::signs_nonce_string_bytes_verifiable`, `backend::pubkey_roundtrips_through_standard_base64`,
`backend::from_db_is_none_when_no_backend_configured` (finding 32), `mcp::config_carries_bearer_auth_header` (finding 12),
`connections::parses_code_and_state_from_callback`, `connections::missing_state_is_none`.

## Device tests (yours - ship does not run device)

Deploy: `make ddev` (or `make all`).

1. NO-REGRESSION (dark path, no backend set): chat works as before, the 3 notes tools (search/create/summarize)
   still answer, no crash. Settings > Privacy: the "Run MCP spike" debug button is GONE.  [T08]
2. Settings now has a "Connections" section. Open it: shows a Backend URL field + "Google Sheets" connector
   listed as "Not connected".  [T09]
3. Desktop (`make desktop`): Connections shows "iOS only", no connect button.  [T09 / finding 29]

The next ones need the OPS preconditions below:

4. Set the Backend URL, tap Connect on Google Sheets: the ASWebAuthenticationSession consent sheet opens, you
   approve, it returns and the status flips to "Connected". Repeat Connect 2-3x: no crash.  [T06 + T05 handshake + T09]
5. Chat: ask the agent to add a row to your sheet -> the MCP tool fires -> a row appears in the sheet.  [T07 + T08]
6. Sad path: disconnect (or stop the backend), ask again -> chat shows the "open Connections / reconnect"
   message; Connections shows "Not connected".  [T13]

## Ops preconditions for tests 4-6 (yours, outward-facing - not code, not in this phase)

- Backend (`marketplace-flowflow`) deployed on Dokploy over HTTPS; paste that URL in Connections.
- Device row marked `premium=1` in the backend DB (authorize/callback/`/v1/mcp` are premium-gated -> 403 otherwise).
- A Google OAuth app registered (`drive.file`), and the backend `GOOGLE_REDIRECT_URI` set to match the callback.

## GATING DECISION before the live connect (test 4) - OAuth callback transport

The device uses callback scheme `flowflow` (const `CALLBACK_SCHEME` in `ui/settings/connections.rs`).
Unresolved tension (RFC OQ#5 / findings 13, 33): a Google **Web** OAuth client (confidential, holds the secret,
which the backend does) requires an **http/https** redirect URI, NOT a custom scheme. A `flowflow://` redirect
needs a Google **iOS** client type (public, PKCE, no secret). Options before T10:
- (A) Google iOS client + custom scheme `flowflow://` (device code already does this); backend brokers via PKCE.
- (B) HTTPS universal-link redirect to the backend (needs the iOS 17.4+ ASWebAuthenticationSession HTTPS callback;
      our deployment target is 16.0, so this needs a target bump or a fallback).
The device wrapper is scheme-parameterized, so only the backend `GOOGLE_REDIRECT_URI` + Google client type + the
`CALLBACK_SCHEME` const need to agree. Decide this when wiring T10 (Phase 3 E2E).

## Out of scope (per contract / RFC Non-Goals)

Monetization/IAP, marketplace, P2P-sync changes, server-side inference, accounts, Android, inbound/push,
Phase 1 backend (shipped), Phase 3 T10 E2E, App Attest, identity recovery.
