---
artifact: "docs/proposals/0002-collaborative-shared-folders/PROPOSAL.md"
run: "T01-T06 (backend)"
repo: "/Users/mirkobozzetto/code/marketplace-flowflow"
branch: "feat/spaces-backend"
base: "dev"
updated: "2026-08-24"
---

# Verification bundle: spaces backend (T01-T06)

## Already run, green

```
cd /Users/mirkobozzetto/code/marketplace-flowflow
cargo fmt                        # applied
cargo clippy --all-targets       # 0 errors, 1 pre-existing warning
cargo test                       # 311 passed, 0 failed (277 before this run)
```

The 34 new tests map to the contract:

| Contract | Test |
|---|---|
| C1 | migration head assertions in `tests/integration.rs` (17 -> 18) |
| C2, C3 | `a_read_ancestor_restricts_the_whole_subtree`, `a_read_ancestor_blocks_a_member_write_in_a_collab_child`, `owner_writes_anywhere_member_only_in_collab`, `a_member_never_edits_another_authors_note` |
| C4 | `a_chain_deeper_than_the_cap_is_refused`, `a_cycle_is_refused_not_looped`, `the_depth_cap_and_the_cycle_refusal_hold_over_http` |
| C5 | `creating_a_space_needs_premium_and_a_linked_identity`, `a_free_guest_joins_and_pulls_a_paid_space` |
| C6 | `unknown_never_joined_and_removed_all_answer_404` |
| C7 | `an_invite_code_works_once`, `an_expired_invite_is_refused` |
| C8 | `rejoining_reuses_the_member_row`, `only_the_owner_invites_or_removes`, `the_owner_cannot_leave_their_own_space` |
| C9, C11 | `deleting_a_folder_tombstones_its_whole_subtree`, `a_deleted_note_reaches_the_other_member_as_a_tombstone` |
| C10 | `every_write_takes_a_distinct_increasing_seq` |
| C12 | `a_truncated_pull_reports_more_and_resumes_without_a_gap` |
| C13 | `a_note_over_64kb_is_refused`, `the_member_cap_refuses_the_21st_without_burning_the_code` |
| C14 | `a_second_pull_inside_the_floor_is_rate_limited`, `the_floor_is_per_space_not_per_device` |
| C15 | `an_unpaid_owner_freezes_writes_but_pull_keeps_serving` |
| leave | `leaving_with_withdrawal_tombstones_only_ones_own_notes` |

## For Mirko to run

Nothing has touched a live database. Migration 18 applies on the next boot
of the backend, and only then.

```
cd /Users/mirkobozzetto/code/marketplace-flowflow
cargo run                        # applies migration 18 to the local db
sqlite3 <db> "SELECT version FROM schema_version"   # expect 18
```

Deploying to `api.flowflow.be` runs the same migration against production
data. It is additive only (5 new tables, 6 new indexes, no ALTER on an
existing table), so a rollback is a `DROP TABLE` of the five, not a data
migration.

## Not covered by this run

- Nothing exercises the routes from a real device: the app-side client is
  T08, a separate run in the `flowflow` repo.
- T16 (two-device / two-account protocol, zero ghost note after withdrawal)
  depends on every app task and stays open.
- Section 8 questions still open: Q1 (forced removal of a revoked member's
  notes - legal), Q4 (does a voice note carry its audio), Q5 (moderation at
  space scale), Q6 (a note whose author left without withdrawing).

## Behavioral choices this run made, worth a second look

- Structural edits to a folder (rename, mode change, move, delete) require
  the folder's AUTHOR or the space owner. Write rights say who may put
  things IN a folder; the proposal does not say who may reshape it.
- The space root is owner-only for members: a member files notes in
  folders, never at the top level.
- `leave` is refused to the owner (`owner_cannot_leave`) rather than
  transferring the space: ownership transfer is not in the proposal.
- A refused join does not spend the invite code. A perfect race can still
  admit one member past the cap; the cap is a cost ceiling, not a security
  boundary.
