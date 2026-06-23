---
type: trace
rfc: 0010-marketplace-of-agents
backend_repo: /Users/mirkobozzetto/code/marketplace-flowflow
branch: feat/marketplace-agents-phase1
base: feat/admin-console-q15 (== origin/main for src/)
updated: 2026-06-23
---

# Ship trace - RFC 0010 Phase 1 (backend core)

## Scope this run (authorized)
Backend core in marketplace-flowflow, independent of the Strata decision:
M1.2 (agent catalog CRUD), M1.3 (V5 account-keyed overrides + writer),
M1.4 (GET /v1/agents), M1.6 (agent-scoped gate), M1.9 (merge guard), M1.8 (tests).
Deferred: M1.1 (Strata, redesigned by spike), M1.5 (on-device, other repo),
M1.7 (admin console, needs OQ7).

## State ledger
| Task | Status | Notes |
|------|--------|-------|
| setup | done | branch feat/marketplace-agents-phase1 off HEAD; serve.ts left dirty, untouched |
| M1.0 spike | DONE | open-strata = STATIC creds per child, NO per-request pass-through. A1-as-auth-proxy REJECTED. RFC OQ1/Decision1/M1.1 amended. Repro in /tmp/strata-repro. |
| ctx map | done | read db.rs catalog.rs admin.rs gate.rs proxy.rs account.rs state.rs util.rs error.rs lib.rs |
| M1.2 | DONE | agent catalog admin CRUD (list/upsert) + validation (alias slug, tools->existing connector, SSRF host allowlist, https endpoints, secret-as-env-name); GET/POST /v1/admin/catalog; hidden-by-default |
| M1.3 | DONE | V5 migration (device->account, revoke-wins dedup, NULL drop, forward-only); POST/DELETE /v1/admin/overrides; entitled_ids re-keyed to account via account_of() |
| M1.4 | DONE | GET /v1/agents (entitled-only, materialized AgentView) |
| M1.6 | DONE | proxy x-agent-id required on slug route; is_accessible(agent) + slug in agent.tools; dial-time host allowlist; legacy /v1/mcp alias keeps bare check |
| M1.9 | DONE | join merge guard: refuse 409 if target/old account holds an agent grant override |
| M1.8 | DONE | 4 new integration tests + 3 existing tests updated for the new contract |
| cargo | GREEN | build 0 warnings; 38 tests pass; fmt + clippy clean |
| review | DONE | adversarial workflow: 0 BLOCKER/MAJOR confirmed. 3 MINOR fixes applied: set_override grants restricted to kind=agent; orphan overrides purged on leave/merge; url_host rejects userinfo. +2 regression tests (V5 non-empty backfill, hidden-connector deny). |
| final | SHIPPED | 8 files, +989/-56. Uncommitted on feat/marketplace-agents-phase1. NOT committed/pushed (awaiting Mirko). |

## Files changed (src/)
state.rs (mcp_host_allowlist), db.rs (V5 + agent seed), catalog.rs (account_of,
account-keyed overrides, AgentConfig/agent_config, agents_view, valid_alias/
url_host/host_allowed), admin.rs (list/upsert_catalog, set/clear_override,
validators), proxy.rs (agent-scoped gate + dial guard), account.rs (merge
guard), lib.rs (routes). tests/integration.rs (+4 tests, 3 updated).

## Key substrate facts (ground truth)
- catalog_items/plan_items/account_item_overrides already exist (db.rs V1 baseline). overrides device-keyed, no writer.
- entitled_ids = plan bundles U grants - revokes (catalog.rs). active_plans -> gate::is_premium (account-level).
- is_accessible(subject, slug, kind) kind-aware (connector|agent).
- proxy.rs = raw reqwest forwarder, injects x-auth-data per-request (per-device token). NOT rmcp.
- admin: ADMIN_TOKEN -> admin_sessions cookie + x-csrf-token + admin_audit (implemented).
- account.rs join already folds IAP entitlements on merge; we add the agent-grant guard.

## Deviations from RFC text
- Override writer mounted at POST/DELETE /v1/admin/overrides (RFC §6.5 said /v1/admin/entitlements);
  clearer since it writes account_item_overrides, not entitlements. Non-breaking.
- Strata role redesigned per M1.0: discovery-only, auth stays on backend (RFC Decision 1 amended).

## Next
1. Implement the 6 tasks above + cargo check green.
2. Adversarial review (workflow).
3. Present; do NOT commit/push without Mirko's ok.
