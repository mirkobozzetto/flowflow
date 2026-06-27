---
artifact: "docs/rfcs/0014-platform-web-app-accounts-roles-agent-management/RFC.md"
artifact_kind: "rfc"
locked: "2026-06-26"
scope_repo: "/Users/mirkobozzetto/code/marketplace-flowflow"
pass: "P1 chunk 1 (T10 + T11a/T11b, Rev 2)"
---

# Definition of Done: RFC 0014 P1 chunk 1 - passkey web accounts socle

> Immutable target. Rev 2 governs (NOT the base §10 table): base T11 (argon2id + tower-sessions) is
> DEAD; auth is passkey/WebAuthn on a hand-rolled session like `admin_sessions`. Requirement changes
> get a NEW entry; never silently rewrite an existing line.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | V8 forward migration creates `web_users`, `web_user_accounts`, `webauthn_credentials`, `web_sessions`; idempotent, runs after V7, leaves legacy rows untouched | RFC T10 / §6.1 / §6.9 | `cargo test` schema test (tables present, version=8) |
| C2 | `web_users.email` UNIQUE on the normalized (lowercased/trimmed) value; format-validated; no password column | RFC §6.1 / T10 | unit test: normalize + duplicate insert rejected |
| C3 | `web_user_accounts.account_id` is UNIQUE (a device cluster maps to at most one web user - B3) | RFC §6.1 / B3 | schema test: UNIQUE on account_id |
| C4 | `POST /v1/auth/register/begin` returns a WebAuthn creation challenge for a new email; `/finish` persists a `web_users` row + a `webauthn_credentials` row | RFC T11a / §6.9 | bundle: curl begin returns challenge; finish path covered by ceremony-state test |
| C5 | `POST /v1/auth/login/begin` returns an assertion challenge for a known user; `/finish` verifies the assertion and mints a hand-rolled session cookie (httpOnly/Secure/SameSite + csrf, hashed at rest, constant-time compare) | RFC T11b / §6.1 | bundle: curl begin; session mint/verify roundtrip test |
| C6 | `POST /v1/auth/logout` deletes the session row and clears the cookie | RFC §6.9 | bundle: curl logout returns 204 |
| C7 | A `WebUser` session extractor authenticates the `web_session` cookie + csrf-on-writes, mirroring `AdminSession`; expired sessions are rejected + purged | RFC §6.1 (session policy) | session roundtrip + expiry test |
| C8 | No `tower-sessions`, no `argon2`, no password storage anywhere in the new code | RFC B6 / Rev 2 | grep: zero matches in the diff |

## Out of scope (never build) - later P1/P2 chunks, do not touch this pass

- Roles enum + authoring regate + ADMIN_TOKEN sunset (T13, TA1) - next chunk
- Device-vouched account link `/v1/account/link` + grant resolver (T14/TA5) - needs OQ1 first
- Admin logins view + role assignment (T15), web signup/login UI (T16)
- access_requests / quota / consents / login_events tables (T20, TA8, T15)
- Rate-limit on `/v1/auth/*` + lockout + user-enumeration-uniform responses (TA4) - noted as deferred ceiling
- Erasure `DELETE /v1/me` (TA6), approval gate (TA2), model allowlist (TA9)

## Edit scope (authorized files, marketplace-flowflow)

- `src/db.rs` - V8 migration (4 tables + ceremony-state table) + cleanup of expired web_sessions/states
- `src/web_auth.rs` (NEW) - WebAuthn register/login/logout handlers + ceremony-state store + email normalize
- `src/gate.rs` - `WebUser` session extractor + `WEB_COOKIE` const (mirrors `AdminSession`)
- `src/lib.rs` - register the new `/v1/auth/register|login|logout` routes + `pub mod web_auth`
- `Cargo.toml` - add `webauthn-rs = "0.5"`
- `Dockerfile` - add `libssl-dev` (builder) + `libssl3` (runtime) for the openssl pull from webauthn-rs-core
- `tests/web_auth_test.rs` (NEW) - migration/schema, email normalize, session lifecycle, link uniqueness

## Notable decisions (surfaced)

- WebAuthn lib = `webauthn-rs` 0.5.5 (kanidm, MPL-2.0, sole mature server lib; §8 open item resolved).
  Pulls `openssl`/`openssl-sys` -> +2 Dockerfile lines. MPL-2.0 is fine server-side (backend, not distributed).
- RP config (rp_id/rp_origin) derived from `PUBLIC_BASE`, overridable via `RP_ID`/`RP_ORIGIN` env.
- `webauthn_states` table (short-TTL, like `nonces`) holds in-flight ceremony state between begin/finish.
- register/finish auto-mints a session (login-after-register UX).
