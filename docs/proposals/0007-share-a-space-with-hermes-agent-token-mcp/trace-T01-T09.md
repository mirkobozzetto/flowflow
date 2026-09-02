---
artifact: "docs/proposals/0007-share-a-space-with-hermes-agent-token-mcp/PROPOSAL.md"
artifact_kind: "propose"
engine_tier: "solo"
work_branch: "ship/share-a-space-with-hermes-agent-token-mcp"
commit_mode: true
stepsCompleted: [0, 1, 2, 3]
final_status: ""
updated: "2026-09-02"
---

# Trace Ledger: Share a space with Hermes agent token and MCP

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Commit | Notes |
|------|---------------|--------|---------------|--------|--------|-------|
| T01 | C1 | done | `src/features/spaces/{core,mod,routes}.rs` | solo | `255c4df` | `spaces_test`: 25 passed |
| T02 | C2 | done | `src/db/migrations.rs`, `tests/migration_agents_test.rs` | solo | `2b90238` | V18 upgrade and constraints pass |
| T03 | C3 | done | `src/gate.rs`, `src/features/spaces/{agents,mod}.rs` | solo | `f749597` | Missing, malformed, expired, revoked tokens return 404 |
| T04 | C4 | done | `src/features/spaces/{core,perm,repo,routes}.rs`, `tests/spaces_perm_test.rs` | solo | `b82140a` | Agent permissions and placement pass |
| T05 | C5 | done | `src/features/spaces/{agents,routes}.rs`, `src/lib.rs`, `tests/spaces_test.rs` | solo | `7f34f66` | 29 route tests pass; security review clean |
| T06 | C6 | done | `src/{ratelimit,state}.rs`, `src/features/spaces/{core,routes}.rs`, `tests/{ratelimit,spaces_test}.rs` | solo | `7f34f66` | 6 limiter tests and folder cap route pass |
| T07 | C7 | pending | | solo | | Security checkpoint required |
| T08 | C8 | pending | | solo | | |
| T09 | C9 | pending | | solo | | |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-04 | migration and auth | proceeded | User authorized T01-T09 explicitly |
## HALT events

- None.
