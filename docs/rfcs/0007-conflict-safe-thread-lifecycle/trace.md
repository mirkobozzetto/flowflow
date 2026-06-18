---
artifact: "docs/rfcs/0007-conflict-safe-thread-lifecycle/RFC.md"
artifact_kind: "rfc"
engine_tier: "solo"
stepsCompleted: [0, 1, 2, 3, 4, 5, 6]
final_status: "shipped"
updated: "2026-06-18"
---

# Trace Ledger: Conflict-safe thread lifecycle (stop boot collapse)

> Single source of truth for progress. One row per T-id.

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Notes |
|------|---------------|--------|---------------|--------|-------|
| T01 | C1 | done | `src/ui/mod.rs` | solo | removed boot `collapse_singleton_threads()` call (1 line) |
| T02 | C2,C3,C4 | done | `src/services/sync/conflict.rs` | solo | restore reads live original (`get_note`/`folders_for_note`); thread baked into INSERT via `create_text_note_in_thread` (guarded by `get_thread`); folders re-linked post-insert; folder re-link failure rolls back (delete_note + unresolve), conflict stays visible |
| T03 | C5 | done | `src/db/thread_repo.rs` | solo | kept fn, doc note: explicit/local-only, never during sync window (filtering covers display) |
| T04 | C2,C3 | done | `tests/sync_conflict_test.rs` | solo | 4 new tests (inherit folder+thread; propagation to peer; dangling-thread guard F4; flat fallback); 12 pass total |

## Adversarial review (post-ship, ultracode, 7 agents)

4 lenses, each MAJOR/BLOCKER finding adversarially verified (refute-by-default):
- FK + apply-ordering: SAFE. Apply runs `PRAGMA foreign_keys=OFF` for the whole batch (apply.rs:971, before BEGIN), `INSERT OR REPLACE` no FK validation, FK restored ON via Drop guard. Restored note with `thread_id=T` applies on the peer regardless of thread-row arrival order; `thread_id` is ON DELETE SET NULL. Phone-created thread -> arrives + renders on Mac, no spurious conflict, no FK error.
- restore-correctness: SAFE. Self-contained membership, fresh UUID (no id fork), double-tap blocked, flat fallback correct.
- boot-collapse-dependency: SAFE. No path depends on boot collapse; every thread-card surface uses the >=2 filter; collapse fn now test-only.
- test-adequacy: 2 gaps closed (propagation-to-peer + F4 dangling-thread guard).
Both synthesis-flagged "bugs" DISMISSED on verification: (1) "not single tx" = matches own contract C4 (rollback + conflict visible); (2) "reboot regression test" = tautology (boot collapse lives in App(), not Database::open).
Known/accepted MINOR (not fixed): hard crash in the sub-ms window between the committed `resolve_conflict` claim and note create -> conflict marked resolved, nothing restored (losing snapshot intact = recoverable, just gone from UI). A true single tx would close it; deemed not worth the cross-repo rewrite. RFC prose 6.2 still says "single transaction" -> doc/impl drift, contract.md C4 is the authoritative criterion.

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-04 | risk-boundary | none expected | no DB migration, no deletion of user data, no dep change, no public-API break |

## HALT events

- none
