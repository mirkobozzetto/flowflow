---
artifact: /Users/mirkobozzetto/code/flowflow/docs/rfcs/0008-mcp-connectors-oauth-broker-backend/RFC.md
artifact_kind: rfc
engine_tier: solo
scope: [P1.1, T02, P1.2, T12, P1.3]
build_repo: /Users/mirkobozzetto/code/marketplace-flowflow
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: shipped
engine_tier: solo
updated: 2026-06-19
---

# Trace Ledger: RFC 0008 Phase 1 (marketplace-flowflow backend)

> Single source of truth for progress. A fresh session reads ONLY this file (+ research.md) to resume.
> Code lives in /Users/mirkobozzetto/code/marketplace-flowflow. Decision: DIRECT (no Strata) + drive.file
> + backend creates the sheet. Toolchain run by ship: cargo test 12/12, clippy clean, fmt clean.

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Notes |
|------|---------------|--------|---------------|--------|-------|
| P1.1 | C1 | done (code); deploy = Mirko | `compose.yml`, `Dockerfile`, `src/proxy.rs`, `.env.example` | solo | backend = the single gated HTTPS MCP endpoint -> internal google-sheets-mcp-server; injects x-auth-data |
| T02  | C2 | done | `src/auth.rs`, `src/gate.rs`, `src/db.rs`, `src/util.rs` | solo | nonce single-use+TTL+bound; Ed25519 verify; session hashed, bound, rotates on refresh. 3 tests |
| P1.2 | C3 | done | `src/oauth.rs`, `src/gate.rs`, `src/crypto.rs` | solo | session+premium gate; PKCE + session-bound single-use state (f22); AES-GCM refresh at rest (f9); revoke+purge (f19); no token logged |
| T12  | C4 | done | `src/ratelimit.rs`, `src/lib.rs` | solo | per-IP fixed-window; 429; layered on /auth/* + gate. 3 tests |
| P1.3 | C5 | done (code); real-token run = Mirko | `examples/connector_spike.rs`, `src/oauth.rs` | solo | minimal MCP-over-HTTP client (init+tools/call), x-auth-data injection; single-flight refresh (f10) |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-02 | architecture (security/OAuth) | RESOLVED by Mirko | DIRECT (drop Strata) + drive.file + backend creates the sheet (research.md surfaced the Strata mismatch). |
| step-04 | new repo + new deps | proceeded | greenfield repo marketplace-flowflow; deps added (axum/sqlx/ed25519/aes-gcm/reqwest...). Logged, reversible. |
| step-04 | config refactor | proceeded | folded rate-limit into Config with named const defaults (Mirko's request). |

## HALT events

- None outstanding. Earlier halt (Klavis premise mismatch) resolved by the architecture decision.

## Adversarial security review (read-only subagent)

All 6 required RFC findings PASS in code (1 nonce single-use/TTL/bound, 3 session bound+rotate +
tenant isolation, 9 refresh encrypted off-DB key, 10 single-flight refresh, 19 revoke+purge, 22 state
session-bound). SQL all bound-param (no injection), no token logged, no gated route bypass.

Hardening fixed after review (re-tested: 14 pass, clippy clean):
- Unbounded growth of `nonces`/`oauth_states`/`sessions` + the rate-limit map + refresh-lock map ->
  added `db::sweep_expired`, `RateLimiter::sweep`, `AppState::prune_refresh_locks`, run every 60s by a
  background task in `main.rs`; indexed `expires_at`.
- Rate-limit keyed on the TCP peer = the Traefik proxy in deploy (one global bucket). Now keys on the
  proxy-supplied client IP (X-Real-Ip / right-most XFF) when `trust_proxy` (default true).
- Access token was plaintext at rest while refresh was encrypted -> access token now AES-GCM encrypted
  too (`access_token_enc` BLOB).
- `verify` -> `verify_strict` (rejects non-canonical sigs / small-order keys).
- Upstream error detail no longer echoed to the client (generic 502 + server-side log).
- Token refresh purged the connection on ANY upstream failure -> now purges ONLY on `invalid_grant`.

Deferred (noted, not built; low value for the single-connector dogfood slice):
- mcp-session-id not bound to device in the proxy (low risk: Sheets data is scoped by the injected
  Google token, not the MCP session id). Revisit in Phase 2 when the device drives MCP sessions.
- No "log out all sessions" (the presented token IS rotated on refresh; old tokens die at their TTL).

## Ops gates remaining (Mirko, outward-facing - ship does not run)

- Register a Google OAuth app (drive.file, Sheets + Drive APIs).
- Deploy on Dokploy (compose + domain + TLS); confirm `/healthz` + the auth handshake live.
- Run the connector spike with a real token (C5) + a manual write (C1).
