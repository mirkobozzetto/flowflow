---
artifact: "docs/proposals/0004-spaces-are-teams-share-into-a-space/PROPOSAL.md"
artifact_kind: "propose"
engine_tier: "solo"
work_branch: "ship/spaces-are-teams"
commit_mode: true
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: "shipped"
updated: "2026-08-25"
---

# Trace Ledger: Spaces are teams, share into a space

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Commit | Notes |
|------|---------------|--------|---------------|--------|--------|-------|
| T00 | server deployed | done | marketplace-flowflow | user | PR #92, #93 | rename route answers 401; deploy now automatic on push to main (secret `DOKPLOY_DEPLOY_WEBHOOK`) |
| T01 | ShareTarget, mode, parent cleared, resume_adoptions | done | `adopt.rs`, `pull.rs`, `space/mod.rs`, `tests/space_share_test.rs` | solo | `b13c53f` | push_subtree shared by share and resume |
| T02 | share-into panel, error line, shared theme edited through the space | done | `folders.rs`, `space_section.rs`, `sidebar/mod.rs`, `fr.ftl`, `en.ftl` | solo | `c60bcf9` | plus: one « Collab » section, separator between teams, empty members hint + invite, drawer bottom padding |
| T03 | Device validation | pending | iPhone | user | | |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-01 | run-gate | overridden | status Review -> Accepted on Mirko's word |
| step-04 | scope | member count not shown in the share panel | one network call per owned space for a number; add if asked |
| step-04 | scope | mode toggle defaults to lock, pencil when no team exists yet | one toggle instead of two defaults |

## HALT events

- none
