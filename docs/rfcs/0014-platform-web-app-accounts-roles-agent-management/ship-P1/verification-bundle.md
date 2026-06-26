---
artifact: "docs/rfcs/0014-platform-web-app-accounts-roles-agent-management/RFC.md"
stack: "rust / cargo"
scope_repo: "/Users/mirkobozzetto/code/marketplace-flowflow"
generated: "2026-06-26"
ran_by: "claude ran cargo; user runs the live/browser checks"
---

# Verification Bundle: RFC 0014 P1 chunk 1 - passkey web accounts socle

> Claude already ran `cargo fmt` + `cargo check --tests` + `cargo test --test web_auth_test` (green).
> What remains is the LIVE validation against a running backend + a real browser passkey - your call.

## Already run by Claude (green)

| Command | Validates | Result |
|---------|-----------|--------|
| `cargo fmt` | formatting | clean |
| `cargo check --tests` | types compile (lib + tests) | exit 0 |
| `cargo test --test web_auth_test` | C1, C2, C3, C6, C7 | 7 tests pass |

Optional extra you may run: `cargo clippy --tests` (style/static), full `cargo test` (whole suite regression).

## Live check (you run, against a running backend - needs a browser for the real ceremony)

The register/login *ceremonies* (C4/C5) cannot be curl'd: the browser's authenticator signs the
challenge. The begin endpoints CAN be smoke-tested for shape; the full ceremony needs the web UI (T16,
a later chunk) or a WebAuthn test harness in the browser devtools.

| Step | Command / action | Expected |
|------|------------------|----------|
| run | `PUBLIC_BASE=https://localhost RP_ORIGIN=http://localhost:8080 RP_ID=localhost cargo run` | server on :8080 |
| C4 begin | `curl -sX POST localhost:8080/v1/auth/register/begin -H 'content-type: application/json' -d '{"email":"a@b.io"}'` | JSON `{ceremony_id, options:{publicKey:{challenge,...}}}` |
| bad email | same with `-d '{"email":"nope"}'` | 400 `{"error":"invalid email"}` |
| C5 begin | `curl -sX POST localhost:8080/v1/auth/login/begin -H 'content-type: application/json' -d '{"email":"unknown@b.io"}'` | 401 (no such user) |
| C6 logout | `curl -isX POST localhost:8080/v1/auth/logout` | 401 (no cookie) |

Full register->login round trip = drive `navigator.credentials.create/get` from a browser against those
endpoints (or a virtual authenticator in Chrome devtools). Lands naturally with the T16 web UI.

## Deploy (USER ONLY: outward-facing, your call)

| Action | Validates | Warning |
|--------|-----------|---------|
| merge + Dokploy redeploy | V8 migration runs on prod app.db; webauthn endpoints live | Dockerfile now needs `libssl-dev`/`libssl3` (added); set `RP_ID`/`RP_ORIGIN` to the real admin web origin before public use |

## Contract coverage

- C1 (V8 tables) -> `v8_creates_passkey_tables`
- C2 (email UNIQUE + normalize) -> `email_normalizes_and_validates` + `web_users_email_is_unique`
- C3 (account_id UNIQUE / B3) -> `web_user_accounts_account_id_is_unique`
- C4 (register begin/finish) -> begin: live curl; finish: covered by compile + ceremony-state store; full E2E needs a browser authenticator
- C5 (login begin/finish + session) -> begin: live curl; session mint/verify: `logout_clears_a_valid_web_session`
- C6 (logout) -> `logout_clears_a_valid_web_session` + `logout_without_cookie_is_unauthorized`
- C7 (WebUser extractor + expiry) -> `expired_web_session_is_rejected_and_purged` + the logout tests
- C8 (no tower-sessions/argon2/password) -> grep the diff: zero matches
- Uncovered by automated test (manual/browser): the cryptographic register->login round trip (C4/C5 finish)
