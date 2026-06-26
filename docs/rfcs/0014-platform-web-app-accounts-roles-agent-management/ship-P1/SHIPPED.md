# SHIPPED - RFC 0014 P1, chunk 1 of 5

Date: 2026-06-26
Scope repo: marketplace-flowflow (branch `feat/rfc0014-p1-passkey-auth`)

This marks ONLY the first P1 chunk done, NOT the whole RFC (P1 has 4 more chunks; P2-P4 untouched).

## Done (T10 + T11a + T11b, Revision 2)

- V8 migration: `web_users`, `web_user_accounts` (account_id UNIQUE - B3), `webauthn_credentials`,
  `web_sessions`, `webauthn_states`; expired-row sweep extended.
- WebAuthn register ceremony (`/v1/auth/register/begin|finish`) -> persists web_user + passkey, auto-login.
- WebAuthn login ceremony (`/v1/auth/login/begin|finish`) + hand-rolled session (httpOnly cookie + csrf +
  constant-time, like `admin_sessions`) + `/v1/auth/logout` + `WebUser` extractor.
- No tower-sessions, no argon2, no password (B6). webauthn-rs 0.5 (+openssl -> 2 Dockerfile lines).

## Verified

cargo fmt + check + clippy + full suite (114 tests pass, 0 fail). The cryptographic register->login round
trip needs a browser authenticator and is NOT yet exercised - lands with the T16 web UI.

## Next chunks (per handoff)

2. T13 (roles enum + authoring regate)
3. TA1 (`/v1/admin/bootstrap` first-admin + ADMIN_TOKEN sunset)
4. T14 + TA5 (device-vouched link; resolve OQ1 first)
5. T16 + T15 (web signup/login UI + admin logins/roles view)
