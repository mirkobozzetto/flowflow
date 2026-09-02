---
artifact: "docs/proposals/0007-share-a-space-with-hermes-agent-token-mcp/PROPOSAL.md"
artifact_kind: "propose"
locked: "2026-09-02"
scope: "T01-T09"
---

# Definition of Done: Share a space with Hermes agent token and MCP

## Acceptance criteria

| # | Criterion | Source | Validated by |
|---|---|---|---|
| C1 | Existing space routes keep their HTTP contracts after extraction. | T01 | Existing spaces tests |
| C2 | Additive migration preserves populated rows and enforces agent author integrity. | T02 | Migration test |
| C3 | Invalid, expired, revoked, or malformed agent tokens return uniform 404. | T03 | Targeted auth tests |
| C4 | Agents follow member write and author-only edit rules with space isolation. | T04 | Cross-space tests |
| C5 | Premium owners create, rotate, list, and revoke up to five agents. | T05 | Route tests |
| C6 | Agents and humans cannot exceed the folder or write-rate limits. | T06 | Limit tests |
| C7 | MCP exposes nine scoped tools, bounded pages, and metadata-only audit logs. | T07 | MCP integration tests |
| C8 | Device pull observes an idempotent agent write; acknowledgement survives and revocation denies access. | T08 | End-to-end integration test |
| C9 | Operators can mint, rotate, revoke, and configure Hermes safely. | T09 | Documentation review |

## Out of scope

- FlowFlow app UI and i18n changes (T10-T12).
- VPS connectivity, Hermes configuration, and cron setup (T13-T15).
- Sync, pull, vector, or RAG behavior changes.

## Edit scope

- ../marketplace-flowflow/src/db/migrations.rs
- ../marketplace-flowflow/src/features/spaces/{core,routes,perm,repo,agents}.rs
- ../marketplace-flowflow/src/features/mcp_spaces/{mod,transport}.rs
- ../marketplace-flowflow/src/{gate,ratelimit,state,lib}.rs
- ../marketplace-flowflow/tests/{migration_agents_test,integration,spaces_test,ratelimit}.rs
- ../marketplace-flowflow/docs/mcp-spaces.md
