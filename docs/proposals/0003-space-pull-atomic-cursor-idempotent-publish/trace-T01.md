---
artifact: "docs/proposals/0003-space-pull-atomic-cursor-idempotent-publish/PROPOSAL.md"
artifact_kind: "propose"
run: "-T01"
repo: "/Users/mirkobozzetto/code/marketplace-flowflow"
engine_tier: "solo"
work_branch: "ship/0003-space-client-id"
commit_mode: true
stepsCompleted: [0, 1, 2, 3, 4]
final_status: ""
updated: "2026-08-25"
---

# Trace Ledger: 0003 run T01 (server)

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Commit | Notes |
|------|---------------|--------|---------------|--------|--------|-------|
| T01 | C1-C5 | done | `src/features/spaces/routes.rs`, `src/features/spaces/repo.rs`, `tests/spaces_test.rs` | solo | `7c5f97d` | `folder_ident`/`note_ident` by PK; `check_id` (<=64 printable ASCII); 201 only on a client-chosen id; folder other-author live same-space stays 403 (existing contract), notes 404 |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-02 | branch | `ship/0003-space-client-id` from `main` | spec spans two repos; this run = server slice |
| step-01 | open question | Q1 (UI pending badge) blocks nothing | logged, not asked |

## HALT events

- none
