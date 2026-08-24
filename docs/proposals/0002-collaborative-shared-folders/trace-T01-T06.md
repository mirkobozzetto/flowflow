---
artifact: "docs/proposals/0002-collaborative-shared-folders/PROPOSAL.md"
artifact_kind: "propose"
engine_tier: "solo"
repo: "/Users/mirkobozzetto/code/marketplace-flowflow"
work_branch: "feat/spaces-backend"
base_branch: "dev"
commit_mode: true
stepsCompleted: [0, 1, 2, 3, 4]
final_status: "shipped"
updated: "2026-08-24"
---

# Trace Ledger: spaces backend (proposal 0002, T01-T06)

> Single source of truth for progress. A fresh session reads ONLY this file to
> resume. Run scoped to T01-T06; T07-T16 are a separate run in the `flowflow` repo.

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Commit | Notes |
|------|---------------|--------|---------------|--------|--------|-------|
| T01 | C1 | done | `src/db/migrations.rs`, `tests/integration.rs` | solo | `2ee03e1` | migration 18, 5 tables + 6 indexes; head assertions 17 -> 18 |
| T02 | C2, C3, C4 | done | `src/features/spaces/perm.rs`, `tests/spaces_perm_test.rs` | solo | `0d1753e`, `a4cb13e` | pure `Tree`; 13 standalone tests |
| T04 | C9, C10, C11 | done | `src/features/spaces/{routes,repo}.rs` | solo | `0d1753e` | folder/move/delete + note/delete; `seq` via `UPDATE ... RETURNING` inside the write's own tx |
| T05 | C12 | done | `src/features/spaces/{routes,repo}.rs` | solo | `0d1753e` | pull paginated at 200/stream, cursor cut to the lower of the two truncated streams |
| T03 | C5, C6, C7, C8 | done | `src/features/spaces/routes.rs`, `src/lib.rs`, `src/gate.rs` | solo | `0d1753e`, `be66e86` | 11 routes mounted; `is_account_premium` extracted |
| T06 | C13, C14, C15 | done | `src/features/spaces/mod.rs`, `src/state.rs`, `src/ratelimit.rs` | solo | `0d1753e`, `be66e86` | caps 20/5000/64 KB; `PullThrottle` 30 s per (device, space); spaces share the content IP bucket |

| tests | C2-C15 | done | `tests/spaces_perm_test.rs`, `tests/spaces_test.rs` | solo | `a4cb13e`, `247a9c2` | 13 permission + 21 route tests |

Suite: 311 passed, 0 failed (was 277 before this run).
`cargo clippy --all-targets` clean apart from one pre-existing warning in
`connector_manifests_test.rs`.

Order is topological: T04/T05 land before T03 because both write
`routes.rs`/`repo.rs` and T03 only mounts the surface.

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-01 | open questions | resolved | Q2 (no space expiry, read-only on premium loss) and Q3 (caps 20/5000/64 KB) answered by Mirko before the run |
| step-02 | branch | `feat/spaces-backend` from `dev` | `dev` recreated from `origin/main` (PR #87 already merged; the stale local `development` predates the SRP refactor and has no `src/features/`) |
| step-04 | risk-boundary | code written, not applied | Migration 18 exists in code only. It runs on the next boot of the backend; no live database has been touched by this run. |
| step-04 | impact | proceeded | `impact(is_premium)` = HIGH (3 direct callers). Signature and behavior unchanged: the entitlement predicate moved into `is_account_premium` and `is_premium` delegates. Suite green. |

## HALT events

- none
