# Verification Bundle: Conflict-safe thread lifecycle (RFC 0007)

## SAFE checks (already run by ship)

| Command | Validates | Result |
|---------|-----------|--------|
| `cargo fmt` | formatting | done (reverted unrelated `tests/thread_test.rs` drift) |
| `cargo clippy --features mobile` | host build, all edits | No issues found |
| `cargo check --features mobile --target aarch64-apple-ios-sim` | iOS path compiles | Finished |
| `cargo test --features mobile --test sync_conflict_test` | restore inheritance, peer propagation, F4 guard, flat fallback (C2, C3) | 12 passed (4 new) |
| `cargo test --features mobile --test thread_test` | collapse fn intact, no regression (C1, C5) | 11 passed |

To install on device: `make ddev` (hot reload device) or `make all`.

## Live test (USER-run, on device) - C7

The real acceptance. Reproduces the original screenshot scenario.

1. Pair desktop (`make desktop`) + iOS app, same WiFi.
2. On iPhone: create a note, drop it in a folder, add it to a thread (>=2 notes).
3. Sync both ways once.
4. Reboot BOTH apps (kill + reopen) a few times, syncing frequently iPhone->Mac.
   - Pass signal: NO `thread <uuid> = null` conflict appears, no member-note
     conflict spawned on its own. (Before this RFC: boot collapse fired the pair.)
5. If a conflict from a PAST collapse is still queued (N4), restore it:
   - Pass signal: the restored note comes back IN its folder and IN its thread,
     not detached.

## Contract coverage

| Criterion | Covered by |
|-----------|-----------|
| C1 no boot collapse, singleton still flat | read-back `ui/mod.rs` (call removed) + thread_test (filter unchanged) |
| C2 restore inherits folder + thread | `restore_conflict_inherits_folder_and_thread` PASS |
| C3 flat fallback when original gone | `restore_conflict_falls_back_to_flat_when_original_gone` PASS |
| C4 rollback on re-link failure, conflict stays visible | read-back `conflict.rs` (delete_note + unresolve on folder re-link Err) |
| C5 collapse fn fate (kept, doc note) | read-back `thread_repo.rs` |
| C6 builds desktop + iOS-sim, suites pass | clippy + iOS-sim check + 2 test suites |
| C7 device: zero spurious conflicts + restore keeps structure | Live test above (manual) |

## Notes / deliberate skips

- T04(a) "simulate boot collapse path emits no tombstone": skipped. Boot collapse
  logic is inline in the `App` component, not a unit-testable seam; extracting it
  would be over-engineering. The regression is gated by (1) the call being removed
  (read-back) and (2) the device double-reboot test (C7). The collapse fn itself
  stays covered by the existing thread_test.
- Chunk restore stays best-effort (re-embed on next edit), unchanged from the
  shipped design. Only the folder/thread re-link gets rollback, because a detached
  note does NOT self-heal whereas missing chunks do.
