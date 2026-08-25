---
artifact: "docs/proposals/0003-space-pull-atomic-cursor-idempotent-publish/PROPOSAL.md"
artifact_kind: "propose"
run: "-T01"
repo: "/Users/mirkobozzetto/code/marketplace-flowflow"
locked: "2026-08-25"
---

# Definition of Done: 0003 run T01 (server: client-chosen ids)

> Immutable target for this scoped run. Requirement changes get a NEW entry.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | `POST /v1/spaces/note` and `/folder` with an unknown `id` create with that id, answer 201, echo the id | PROPOSAL.md T01 | `tests/spaces_test.rs::a_client_id_creates_once_and_replays_as_an_update` |
| C2 | Replaying the same `id` (own, live, same space) updates: one row, 200 | PROPOSAL.md T01 | same test, `note_count == 1` |
| C3 | Tombstoned / other-space / other-author id answers 404 | PROPOSAL.md T01 | `a_dead_foreign_or_other_space_id_answers_404` |
| C4 | `MAX_NOTES` is checked on the create branch only | PROPOSAL.md T01 | `the_note_cap_applies_to_creates_only` |
| C5 | Absent `id` keeps today's behaviour (server-minted id, 200) for 2.0.1 clients | PROPOSAL.md §6.2 | same test, last block; all pre-existing route tests unchanged |

## Out of scope (never build)

- Local conflict resolution, folder write queue, delta format or pagination change, dedup of existing duplicates, embed batching (PROPOSAL.md §4).
- Deploy (T02) is the user's.

## Edit scope

- `marketplace-flowflow/src/features/spaces/routes.rs`
- `marketplace-flowflow/src/features/spaces/repo.rs`
- `marketplace-flowflow/tests/spaces_test.rs`
