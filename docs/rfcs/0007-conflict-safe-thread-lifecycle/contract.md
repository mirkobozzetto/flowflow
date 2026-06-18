---
artifact: "docs/rfcs/0007-conflict-safe-thread-lifecycle/RFC.md"
artifact_kind: "rfc"
locked: "2026-06-18"
---

# Definition of Done: Conflict-safe thread lifecycle (stop boot collapse)

> Immutable target. Requirement changes get a NEW entry; never silently rewrite.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | `collapse_singleton_threads()` is no longer called at boot; a `<2`-member thread's note still shows flat (existing query filter) | RFC T01 | read-back `ui/mod.rs`; existing `list_root_notes*` filter unchanged |
| C2 | `restore_note_conflict` inherits the surviving original's (`get_note(entity_id)`) `thread_id` (only if `get_thread` alive) and all `folders_for_note(entity_id)` links onto the restored note | RFC T02 | unit test (restore inherits) |
| C3 | If the original note is gone, the restored note is flat (no folder, no thread); no error | RFC T02 / D3 | unit test (flat fallback) |
| C4 | Restore rolls back fully on a re-link failure, leaving the conflict visible (no detached note behind a resolved conflict) | RFC T02 / F2 | read-back `conflict.rs` (rollback path) |
| C5 | `collapse_singleton_threads` fate decided: kept with a doc note (explicit/local only, never during sync), no dead auto-trigger | RFC T03 / Q1 | read-back `thread_repo.rs` |
| C6 | Builds desktop (host) + iOS-sim; existing collapse + conflict tests still pass | RFC verification | `cargo clippy` + `cargo check --target aarch64-apple-ios-sim` (user-run bundle) |
| C7 | iPhone<->Mac: 2-note thread, sync both ways, reboot both -> zero spurious conflicts; restore a conflicted foldered+threaded note -> same folder + thread | RFC verification (device) | manual device test (bundle) |

## Out of scope (never build)

- N1: redesigning conflict resolution / version-vector semantics
- N2: GC of orphan (0-1 member) thread rows (deferred, harmless, filtered)
- N3: sync timeout / peer-discovery (issue #58, separate)
- N4: retroactively cleaning conflicts already queued from past collapses

## Edit scope

- `src/ui/mod.rs` (remove boot call)
- `src/services/sync/conflict.rs` (restore inherits folder + thread)
- `src/db/thread_repo.rs` (collapse fn doc note)
- `tests/sync_conflict_test.rs` (new tests)
