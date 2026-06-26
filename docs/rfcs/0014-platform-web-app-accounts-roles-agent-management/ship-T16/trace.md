---
artifact: "docs/rfcs/0014-platform-web-app-accounts-roles-agent-management/RFC.md"
artifact_kind: "rfc"
pass: "T16 - passkey signup/login UI + become-admin"
scope_repo: "/Users/mirkobozzetto/code/marketplace-flowflow"
branch: "feat/rfc0014-p1-passkey-auth"
engine: "solo"
status: "shipped (toolchain green; browser ceremony pending device validation)"
date: "2026-06-27"
---

# Trace - RFC 0014 T16

## Units

| # | Unit | Files | Status |
|---|------|-------|--------|
| 1 | Backend `GET /v1/auth/me` (profile + role + session csrf, behind `WebUser`) | `src/web_auth.rs`, `src/lib.rs` | done |
| 2 | `/v1/auth/*` same-origin proxy (cookie first-party; was admin-only) | `admin/src/routes/v1/auth/$.ts` (new) | done |
| 3 | Passkey client (register/login/logout/me + becomeAdmin) via `@simplewebauthn/browser` | `admin/src/lib/api.ts`, `admin/package.json` | done |
| 4 | `/` rewritten: ADMIN_TOKEN paste -> passkey signup/login (shadcn Card, brand mark) | `admin/src/routes/index.tsx` | done |
| 5 | `/account` (new): profile + role badge + logout + "Become admin" bootstrap | `admin/src/routes/account.tsx` (new) | done |

## Toolchain (run by Claude)

- backend `cargo fmt` + `cargo check`: clean (exit 0)
- admin `bun install` (@simplewebauthn/browser ^13): ok
- admin `bun run check` (biome): clean
- admin `bun run build` (vite): ok; routeTree regenerated with `/account` + `/v1/auth/$`
- admin `bun run typecheck` (tsc --noEmit): exit 0
- local run: backend `listening on :8080`, admin vite `:3000`, both booted

## Checkpoints

- New dependency `@simplewebauthn/browser` ^13 added (security-path browser ceremony; named in the
  contract). Logged, not gated - forced by the passkey requirement.
- No DB migration in this pass (V8 tables already shipped). No destructive op.

## Pending (user-run, needs a real authenticator)

- Browser ceremony on a passkey device: register -> reload -> logout -> login -> become-admin -> dashboard.
  Steps in verification-bundle.md. This is the only part Claude cannot self-run.
- Residual risk to watch: webauthn-rs 0.5 <-> @simplewebauthn/browser JSON field compat at the actual
  ceremony (unwrap of `options.publicKey` confirmed as the standard pairing; runtime proof is the device test).
