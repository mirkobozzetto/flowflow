---
artifact: "docs/proposals/0003-space-pull-atomic-cursor-idempotent-publish/PROPOSAL.md"
artifact_kind: "propose"
run: "-T03-T08"
repo: "/Users/mirkobozzetto/code/flowflow"
engine_tier: "solo"
work_branch: "ship/0003-space-pull-atomic"
commit_mode: true
stepsCompleted: [0, 1, 2, 3, 4, 5, 6]
final_status: "shipped"
updated: "2026-08-25"
---

# Trace Ledger: 0003 run T03-T08 (client)

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Commit | Notes |
|------|---------------|--------|---------------|--------|--------|-------|
| T03 | C1 | done | `persistence/{folder,note,settings,share,space}_repo.rs`, `persistence/mod.rs`, `note_persistence.rs` | solo | `e41737a` | `DbTx` in space_repo.rs; 25 methods moved, Database delegates; 6 methods on SAVEPOINT (the 4 listed + `delete_share`, `delete_provenance`, both reached by `delete_note`); `with_tx`; `create_note_in` / `delete_note_rows` / `finish_note_delete` |
| T06 | C4 | done | `schema.rs`, `space_repo.rs`, `tests/space_schema_test.rs`, `domain/space.rs` | solo | `e41737a` | V27 + index; repo `stage/clear/defer/due/note_publish_state`; `publish_backoff` rule in domain; committed with T03 (same files) |
| T04 | C2 | done | `pull.rs`, `tests/space_delta_test.rs`, `tests/space_leave_test.rs` | solo | `2211091` | every `let _` / `else continue` on SQL is `?`; tests go through `apply_space_page` |
| T05 | C3 | done | `space_repo.rs`, `pull.rs` | solo | `e41737a`, `2211091` | `apply_space_page` (BEGIN IMMEDIATE, cursor in tx, ROLLBACK on error); `PageEffects` run after commit (audio files, purge queue, embed, drain) |
| T07 | C5 | done | `write.rs` | solo | `bbd1f61` | client id on note + folder create; `publish_local_note` stages then pushes; permanent = 400/404/409 minus `space_read_only` (so a 409 cap detaches too); local guard / no backend defer, never detach |
| T08 | C6 | done | `pull.rs`, `tests/space_publish_test.rs` | solo | `2211091`, `bbd1f61` | `republish_pending` public, cap 20, at the head of `pull_space`; drain test runs with no backend (every push defers) |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-02 | branch | `ship/0003-space-pull-atomic` from `main` | spec spans two repos; this run = client slice, server slice in trace-T01.md |
| step-01 | dependency | T07 depends on T02 (deploy); coded against the T01 contract, deploy before the next App Store build | server accepts absent id, so a 2.0.1 client keeps working; a new client against the old server would 404 on create |
| step-04 | DB migration | V27 additive, in the spec's plan | no separate ask: the Accepted proposal is the approval |
| step-04 | scope edge | `delete_share` / `delete_provenance` also moved to SAVEPOINT | reached by `delete_note` inside a page; a BEGIN there would abort the page |
| step-04 | commits | 3 commits for 6 units | T03/T06 and T05/T08 share files with their neighbours; a per-unit split would not build |

## Verification (run by ship, project rule: Claude runs the toolchain)

- `cargo test`: exit 0 (space_delta 13, space_leave 4, space_schema 2, space_publish 3, space_client 3, rest unchanged)
- `make check`: fmt clean, clippy only the pre-existing `period_empty` warning in `rag/mod.rs`

## HALT events

- none
