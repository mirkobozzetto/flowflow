---
artifact: RFC 0009 (status Review/Accepted, gate overridden by Mirko)
scope: Q1.7 cutover - retire PREMIUM_PUBKEYS + drop devices.premium (+ §12 C-cut)
engine: solo
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: code complete (32 tests green, clippy clean) - DEPLOY HELD behind the live-200 gate
---

# Ship trace - RFC 0009 Q1.7 cutover

## Definition of done
Retire the two pre-0009 premium bridges so an active account entitlement is the
SOLE premium source, and drop the now-dead `devices.premium` column - all in one
release (F8), without a premium-loss gap (R8 / finding 7) and without breaking
connector access (the §12 C-cut, finding E2).

## Scope (8 files - bigger than "env + column")
- `src/state.rs` - drop `premium_pubkeys` field + `load_premium_pubkeys` + the
  `PREMIUM_PUBKEYS` env read; drop the now-unused `HashSet` import.
- `src/gate.rs` - `is_premium` becomes pure `pubkey -> account -> active
  entitlement`; removed the env allowlist short-circuit AND the `d.premium = 1`
  OR-branch. `PremiumDevice` doc updated.
- `src/auth.rs` - `verify` device INSERT no longer writes `premium` (F8).
- `src/catalog.rs` - C-cut: `active_plans` now derives the `premium` plan from
  `gate::is_premium` (account entitlement) instead of `SELECT premium FROM
  devices`. One source of truth shared with the gate. `subject_id` stays
  device-keyed (overrides table has no writer; re-key only when it gets one).
- `src/admin.rs` - removed `POST /v1/admin/premium` (`set_premium` +
  `SetPremiumReq`); `/v1/admin/devices` listing drops the `premium` field
  (premium is per-account now, read via `/v1/admin/entitlements`).
- `src/lib.rs` - removed the `/v1/admin/premium` route.
- `src/db.rs` - V4 migration: `ALTER TABLE devices DROP COLUMN premium`,
  version-gated + transactional like every forward step. Plain DROP COLUMN
  (bundled SQLite >= 3.35; prod runs the same pinned lib).
- `tests/integration.rs` - `grant_premium` helper now writes an account
  entitlement; deleted `admin_grant_revoke_and_list` + `resolver_env_fallback`
  + `mk_state_premium_env`; `catalog_resolver_matrix` and
  `admin_entitlement_grant_revoke` exercise the entitlement->premium-plan path;
  `migration_baseline_stamps_without_clobber` is now the V4 drop guard (asserts
  schema_version 4 and the `premium` column is gone).

## Checks (Claude, on host)
- `cargo fmt` clean, `cargo clippy --all-targets` clean.
- `cargo test`: 32 passed (was 34; 2 legacy-bridge tests removed).

## DEPLOY GATE - NOT yet deployed (see verification-bundle.md)
V4 is forward-only and drops a column: irreversible. Deploy is HELD until
Mirko's account is confirmed to carry an active entitlement on prod (so removing
the env bridge cannot strip his premium). The bundle is the runbook.
