---
artifact: "docs/proposals/0003-space-pull-atomic-cursor-idempotent-publish/PROPOSAL.md"
artifact_kind: "propose"
run: "-T03-T08"
repo: "/Users/mirkobozzetto/code/flowflow"
locked: "2026-08-25"
---

# Definition of Done: 0003 run T03-T08 (client)

> Immutable target for this scoped run. Requirement changes get a NEW entry.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | `Database` methods delegate to `DbTx`; `delete_note` under an open transaction commits with the outer tx; `cargo test` green | T03 | `space_delta_test::a_full_page_with_deletes_and_moves_commits_as_one`, full suite |
| C2 | `apply_delta(&DbTx) -> Result`, no swallowed SQL error; delta tests pass | T04 | `space_delta_test` (all) |
| C3 | RAISE trigger mid-page: cursor unchanged, no partial row; full page: no deadlock | T05 | `a_page_that_fails_midway_leaves_no_row_and_no_cursor`, `a_full_page_with_deletes_and_moves_commits_as_one` |
| C4 | V27 `space_publish_pending` covered by `space_schema_test`; not in sync catalog | T06 | `space_schema_test` (both tests) |
| C5 | `remote_id` + pending row written in one tx before network; replay uses same id; 400/404/409 detaches; transient sets backoff; folder create sends id | T07 | `space_publish_test::staging_binds_the_remote_id_and_queues_the_note_as_one`, `a_deferred_note_waits_out_its_backoff`; detach/replay paths need a backend: T09 device |
| C6 | `republish_pending` at pull start: pending note published on next pull; cap 20; `next_try_at` honored | T08 | `the_drain_takes_the_due_notes_only_and_at_most_twenty`; the actual push: T09 device |

## Out of scope (never build)

- Local conflict resolution; folder write queue; delta format or pagination change; dedup of existing duplicates; embed batching (PROPOSAL.md §4).
- T02 deploy and T09 device validation are the user's.

## Edit scope

- `src/application/space/{pull,write,mod}.rs`
- `src/application/note_persistence.rs`
- `src/infrastructure/persistence/{space,note,folder,settings,share}_repo.rs`, `mod.rs`, `schema.rs`
- `src/domain/space.rs` (backoff rule)
- `tests/space_{delta,leave,schema,publish}_test.rs`
