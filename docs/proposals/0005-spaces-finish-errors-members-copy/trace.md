---
artifact: "docs/proposals/0005-spaces-finish-errors-members-copy/PROPOSAL.md"
artifact_kind: "propose"
engine_tier: "solo"
work_branch: "ship/spaces-finish-errors-members-copy"
commit_mode: true
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: "shipped"
updated: "2026-08-25"
---

# Trace Ledger: Spaces finish (errors, members, copy)

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Commit | Notes |
|------|---------------|--------|---------------|--------|--------|-------|
| T01 | C1, C2 | done | `space/mod.rs`, `settings/spaces.rs`, `space_section.rs`, `fr.ftl`, `en.ftl`, `tests/space_error_test.rs` | solo | `f5a4864` | `error_key` + 7 keys; `fail` closure in settings, `show` in sidebar |
| T02 | C3, C4, C5 | done | `space_section.rs`, `fr.ftl`, `en.ftl` | solo | `fad47cd` | `status` signal, `open(Panel)` clears status+error, menu tap clears status; `space-member-anonymous` removed |
| T03 | C6 | pending | iPhone | user | | device validation |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-01 | run-gate | overridden | status Review -> Accepted on Mirko's word |
| step-02 | branch | `ship/spaces-finish-errors-members-copy` from `ship/spaces-sidebar` | 0005 builds on the unmerged sidebar branch |
| step-04 | scope | `folders.rs` skipped | its error line comes with 0004, not shipped |

## HALT events

- none
