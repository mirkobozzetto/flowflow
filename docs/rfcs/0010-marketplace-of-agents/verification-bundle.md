---
type: verification-bundle
rfc: 0010-marketplace-of-agents
phase: 1 (backend core)
repo: /Users/mirkobozzetto/code/marketplace-flowflow
branch: feat/marketplace-agents-phase1
status: shipped (uncommitted, pending review + your OK)
---

# Verification bundle - RFC 0010 Phase 1 backend core

All SAFE checks below were already run by Claude and are GREEN. Re-run any to confirm.
Commands use `--manifest-path` so they work from either repo.

## Already run, green
```bash
# build (0 warnings)
cargo build --manifest-path /Users/mirkobozzetto/code/marketplace-flowflow/Cargo.toml
# tests (38 passed, 4 suites)
cargo test  --manifest-path /Users/mirkobozzetto/code/marketplace-flowflow/Cargo.toml
# lint + format (clean)
cargo clippy --manifest-path /Users/mirkobozzetto/code/marketplace-flowflow/Cargo.toml --all-targets
cargo fmt    --manifest-path /Users/mirkobozzetto/code/marketplace-flowflow/Cargo.toml --check
```

## New tests added (M1.8) - what they prove
- `agents_endpoint_filters_to_granted_account` - a granted agent appears on `/v1/agents` for that account only; siblings see nothing (M1.2/M1.3/M1.4).
- `proxy_agent_scoped_gate` - slug route without `x-agent-id` -> 400; unknown agent -> 403; entitled agent that does not declare the connector -> 403 (M1.6).
- `proxy_denies_agent_tool_on_inactive_connector` - an agent declaring a hidden connector is still refused at the proxy (active-only invariant).
- `catalog_upsert_validates_agent` - bad alias -> 400, unknown tool -> 400, valid -> 200 hidden + listed.
- `join_blocked_when_target_has_agent_grant` - join into an account holding an agent grant -> 409 (M1.9 merge guard).
- `v5_rekeys_device_overrides_to_account` - V5 backfill on non-empty data: revoke-wins, dedup, NULL-account drop.
- Updated: `catalog_resolver_matrix`, `resolver_active_expired_unpaired` (overrides now account-keyed), `backward_compat_*` (slug route now needs x-agent-id), `migration_baseline_*` (schema is now V5).

## Manual API smoke (optional, needs a running backend + ADMIN_TOKEN)
```bash
# 1. admin login -> cookie + csrf, then compose an active agent
#    POST /v1/admin/catalog  { id, kind:"agent", display_name, status:"active",
#       config:{ tools:["google"], system_prompt_ref, model, alias } }
# 2. grant it:  POST /v1/admin/overrides { account_id|device_pubkey, catalog_item_id, effect:"grant" }
# 3. as a device:  GET /v1/agents (Bearer)  -> the agent, materialized
# 4. tool call:  POST /v1/connectors/google/mcp  (Bearer + header x-agent-id: <agent id>)
#    - without x-agent-id -> 400 ; non-granted agent -> 403
```

## User-gated (NOT run by Claude - your call)
- `git` commit / push of branch `feat/marketplace-agents-phase1` (git-guard will prompt).
- Docker build + Dokploy deploy of the backend (no schema change needed beyond the V5 migration, which runs on boot; forward-only, empty-table-safe).
- No DB migration to run by hand: `db::migrate` applies V5 on startup.

## Deferred (next phases, NOT in this branch)
- M1.1 Strata as DISCOVERY layer only (auth stays on backend - spike M1.0 verdict).
- M1.5 on-device agent activation (flowflow app repo).
- M1.7 admin compose/test/grant console (needs OQ7: server-side test run shape).
