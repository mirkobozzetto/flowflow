---
artifact: /Users/mirkobozzetto/code/flowflow/docs/rfcs/0008-mcp-connectors-oauth-broker-backend/RFC.md
stack: rust / cargo
build_repo: /Users/mirkobozzetto/code/marketplace-flowflow
generated: 2026-06-19
ran_by: ship ran the safe set; deploy/Google gates are Mirko's
---

# Verification Bundle: RFC 0008 Phase 1 (marketplace-flowflow)

## Safe checks (ship already ran these; re-run to confirm)

| Command | Validates | Result |
|---------|-----------|--------|
| `cd /Users/mirkobozzetto/code/marketplace-flowflow && cargo test` | C2 (nonce/replay/expiry/rotate), C3 (gate 401/403, state binding, disconnect purge), C4 (429), crypto round-trip | 12 passed |
| `cargo clippy --all-targets` | lints | no issues |
| `cargo fmt --check` | formatting | clean |
| `cargo build --release` | release binary builds (also what the Dockerfile runs) | run to confirm |

## Ops / outward-facing (USER ONLY: ship never runs these)

| Step | Validates | Warning |
|------|-----------|---------|
| Register a Google OAuth app (Web, `drive.file`, enable Sheets + Drive APIs) | C5 prerequisite | account-bound; see README |
| `docker run --rm -p 5000:5000 -e SKIP_OAUTH=true ghcr.io/klavis-ai/google-sheets-mcp-server:latest` | the Sheets MCP server is reachable | pulls an external image |
| `GOOGLE_ACCESS_TOKEN=ya29... cargo run --example connector_spike` | C5: a real `drive.file` token creates a sheet + writes a row via `x-auth-data` | writes to YOUR Google Drive |
| Deploy on Dokploy (compose + domain + TLS), then `curl https://<domain>/healthz` | C1: one gated HTTPS MCP endpoint live in front of the internal Sheets server | provisions infra; see README |
| curl `/v1/auth/challenge` -> sign -> `/v1/auth/verify` on the deployed host | C2 on the real deploy | none, but hits the live box |

## Contract coverage

- C1 (one gated HTTPS MCP endpoint -> Sheets, manual write) -> `compose.yml` + `Dockerfile` + `src/proxy.rs` (ship); deploy + manual write (Mirko).
- C2 (nonce single-use/TTL/bound, verify consumes, replay/expired 401, session bound + rotate) -> tests `auth_handshake_replay_and_rotate`, `bad_signature_rejected`, `expired_nonce_rejected`.
- C3 (session+premium gate, state session-bound, disconnect purge, refresh encrypted, no token logged) -> tests `premium_gate`, `oauth_state_must_match_device`, `disconnect_purges_token` + `crypto` round-trip; no-token-logging is a code-review check (see adversarial review).
- C4 (429 on /auth/* + gate) -> tests `rate_limit_returns_429` + `ratelimit` unit tests.
- C5 (real Google token -> create sheet + write row via x-auth-data; single-flight refresh) -> `examples/connector_spike.rs` (Mirko runs with a token); single-flight refresh = `src/oauth.rs::valid_access_token`.

Uncovered by automated tests (need the ops gates above): C1 live deploy, C5 real-token write, full
browser OAuth E2E (that is Phase 2 with the device). The OAuth exchange + refresh code is built and the
state-binding logic is unit-tested; the live token exchange is exercised in Phase 2 / the spike.
