---
artifact: /Users/mirkobozzetto/code/flowflow/docs/rfcs/0008-mcp-connectors-oauth-broker-backend/RFC.md
artifact_kind: rfc
locked: 2026-06-19
scope: [P1.1, T02, P1.2, T12, P1.3]
build_repo: /Users/mirkobozzetto/code/marketplace-flowflow
engine_tier: solo
---

# Definition of Done: RFC 0008 Phase 1 - backend infra (marketplace-flowflow)

> Immutable target. Phase 1 = the thin premium-gate backend (Rust/axum) in FRONT of a self-hosted
> Klavis `open-strata` aggregator + Google Sheets MCP, deployed on Dokploy. Requirement changes get a
> NEW row, never a silent rewrite. Code lands in the NEW repo `marketplace-flowflow`, never in `flowflow`.

## Build vs ops boundary

ship writes code + compose + config + a connector spike, and runs `cargo check`/`cargo build` on the new
repo itself. The DEPLOY (Dokploy), the Google OAuth app registration, and the manual end-to-end Sheets
tests are Mirko's gates (outward-facing / account-bound). The verification bundle lists both halves.

## Acceptance criteria (the contract)

> REVISED 2026-06-19 after pre-build research + Mirko's decision: topology = DIRECT (no Strata in Phase 1);
> the backend owns the Google OAuth broker and injects the token via `x-auth-data` into
> `google-sheets-mcp-server`. Scope = `drive.file` + the backend CREATES the CRM sheet (keeps the CASA dodge).

| # | Criterion (revised) | Source | Validated by |
|---|--------------------------|--------|--------------|
| C1 | The backend exposes ONE gated MCP endpoint over HTTPS that proxies to the internal `google-sheets-mcp-server` (token injected via the `x-auth-data` header); a manual cell write works through it | P1.1 | compose + Dockerfile land + `cargo build` (ship); deploy + manual write (Mirko, ops gate) |
| C2 | `/auth/challenge` nonce bound to pubkey + single-use + TTL; `/auth/verify` consumes the nonce; replayed or expired nonce -> 401; session token bound to pubkey + rotated on refresh (findings 1, 3) | T02 | unit tests (nonce single-use/replay/expiry, Ed25519 verify, session rotate) + read-back |
| C3 | Device session required in front of the proxy endpoint; non-session AND non-premium blocked; OAuth `state` single-use + session-bound (finding 22); disconnect revokes upstream + purges the token row (finding 19); refresh tokens encrypted at rest, off-DB key (finding 9); no token ever logged | P1.2 | unit/integration tests (401 no session, 403 not premium, state-mismatch reject, purge-on-disconnect, encrypt/decrypt round-trip) + grep: no token in any log |
| C4 | Excess requests to `/auth/*` and the gate -> 429 (finding 20) | T12 | integration test (burst -> 429) |
| C5 | Real Google OAuth (`drive.file`) -> the backend creates a CRM sheet then appends a row (`write_to_cell`) via `x-auth-data` injection into `google-sheets-mcp-server`; single-flight token refresh (finding 10); end-to-end token-injection path verified | P1.3 | connector spike bin/test (ship writes; Mirko runs with a real Google app, ops gate) |

## Out of scope (never build)

- All device / `flowflow`-repo tasks: T05, T06, T07, T08, T09, T13, T10 (Phase 2-3).
- The SUPERSEDED GitHub-broker design (T00a/b host-side already done; T01, T02b, T03a/b, T04a/b/c, T11 GitHub variants).
- Monetization / IAP backend metering (separate future RFC).
- Postgres / multi-instance (SQLite single-instance is the v0 store).
- App Attest / DeviceCheck production device attestation (deferred; TOFU + rate-limit for the dogfood slice).
- Identity-recovery path (encrypted key backup / account binding) - documented limitation, not built.
- Full Google `spreadsheets` scope - `drive.file` only; the backend creates the CRM sheet (dodges CASA).
- Klavis Strata aggregator - deferred until 2+ connectors (it neither brokers OAuth nor relays the device
  token in self-host standalone mode, and adds only progressive tool-disclosure for one connector).
- SSE/streaming MCP proxy passthrough - Phase 1 proxies the Sheets server's plain `/mcp` POST leg only.

## Edit scope

New greenfield repo `/Users/mirkobozzetto/code/marketplace-flowflow` only:
- `Cargo.toml`, `src/main.rs` (axum router + layers), `src/state.rs`, `src/error.rs`
- `src/auth/` (nonce store, Ed25519 verify, session mint/rotate) [C2]
- `src/oauth/` (Google authorize/callback + PKCE + single-use session-bound state + single-flight refresh) [C3,C5]
- `src/crypto.rs` (AES-GCM envelope encryption of refresh tokens; master key from env) [C3]
- `src/proxy.rs` (gated passthrough to internal `google-sheets-mcp-server`, injects `x-auth-data`) [C1]
- `src/gate.rs` (session + premium middleware, revoke/disconnect) [C3]
- `src/ratelimit.rs` (tower layer on `/auth/*` + the gate) [C4]
- `src/db.rs` + SQLite migrations (`devices`+premium flag, `sessions`, `nonces`, `connector_tokens`, `oauth_states`)
- `compose.yml` (backend public + `google-sheets-mcp-server` internal) + backend `Dockerfile` [C1]
- `examples/connector_spike.rs` (real Google token -> create sheet -> write row via injection) [C5]
- `README.md` / deploy notes, `.env.example`

NO edits to the `flowflow` app repo. NO edits to the Accepted `RFC.md` (progress -> `ship/phase1/trace.md`).

## Decisions (resolved 2026-06-19)

- **Topology:** DIRECT - no Strata in Phase 1. Backend owns Google OAuth + refresh + token injection.
- **Scope:** `drive.file`; the backend creates the CRM sheet so write access holds without a sensitive scope.
- **Premium check (P1.2):** `premium` boolean column on `devices` (default false, flippable for dogfood);
  gate returns 403 when the session is valid but `premium = false`. IAP-receipt validation deferred.
