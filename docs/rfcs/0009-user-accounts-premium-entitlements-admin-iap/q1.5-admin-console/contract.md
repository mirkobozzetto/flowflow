---
artifact: docs/rfcs/0009-user-accounts-premium-entitlements-admin-iap/RFC.md
artifact_kind: rfc
scope: Q1.5 - TanStack Start + shadcn admin shell over /v1/admin
locked: 2026-06-22
---

# Definition of Done: RFC 0009 Q1.5 admin console

> Immutable target. Thin React shell over the LIVE Rust admin API. Zero
> backend change. New code lives in `marketplace-flowflow/admin/`.

## Acceptance criteria (the contract)

| # | Criterion | Source | Validated by |
|---|-----------|--------|--------------|
| C1 | TanStack Start + Bun + shadcn project, TS strict, Biome lineWidth 80; runs on `bun dev` | RFC §6ter, user | `bun install && bun dev`, `bun run typecheck`, `bun run check` |
| C2 | Login exchanges ADMIN_TOKEN at POST /v1/admin/login, caches csrf, NO auth library | RFC §6ter, admin.rs login | manual: paste token -> dashboard |
| C3 | Same-origin proxy forwards /v1/admin/* to the Rust backend (cookie + csrf + Set-Cookie passthrough); no CORS, no backend edit | RFC §6ter "thin React over the Rust admin API" | network tab: calls hit :3000, cookie set on :3000 |
| C4 | Devices screen lists GET /v1/admin/devices (device_id, account_id, last_seen) | admin.rs list_devices | manual: Devices table populated |
| C5 | Entitlements screen lists GET /v1/admin/entitlements | admin.rs list_entitlements | manual: Entitlements table populated |
| C6 | Grant: POST /v1/admin/entitlements/grant by account_id XOR device_pubkey, plan default premium, optional expiry, x-csrf-token | admin.rs grant | manual: grant -> row active |
| C7 | Revoke: POST /v1/admin/entitlements/revoke with x-csrf-token | admin.rs revoke | manual: revoke -> row revoked |
| C8 | 401 from any call clears csrf and returns to login | admin.rs AdminSession | manual: expire/clear -> login |

## Out of scope (never build here)

- Catalog screen (§12 C6): no admin catalog endpoint exists on the
  backend; nothing to wire. Deferred until the backend exposes it.
- Accounts-list endpoint: not mounted; accounts are reached via the
  devices list (device -> account_id) and entitlements list.
- Any auth library (Better Auth etc.) - dropped by the §6ter pivot.
- Apple IAP, account join/leave (those are app/backend tasks).
- Backend changes of any kind (CORS, routes, schema).
- Production deploy / Dockerfile - follow-up after dev validation.

## Edit scope

- `marketplace-flowflow/admin/**` (new directory only).
- The Rust backend (`marketplace-flowflow/src/**`) is READ-ONLY here.
