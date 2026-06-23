# Vision: Marketplace of agents (locked mental model)

Status: vision brief, not an RFC yet. Captured to avoid drift. English per repo convention.

## Product model

- Users create an account on the web app to use the marketplace. No account = no access.
- Today everything in the marketplace is premium; nothing is free yet. A free tier may open later.
- The owner (Mirko) grants access per account: either things made available, or bespoke tools built on demand for one specific user, personal and exclusive to them.
- Access flows through an entry door from the mobile app to the web app: the user signs in, and that connection delivers the authorizations that unlock features. Those features only work while the account link exists.
- What gets granted are MCP connectors/tools that turn the iOS/macOS app into a controller of external pro tools: CRM, third-party apps, n8n automations, chained actions.

## How a granted tool fires

- A note that expresses a need is saved as a note AND executes that need through a granted tool.
- Programmable triggers map a phrase to an action: saying or writing "lance xxx" executes xxx (voice or text).
- This is the same pattern already proven by RFC 0003 (a note creates a calendar event), generalized to any marketplace tool.

## Quality principle (non-negotiable)

- Nothing is granted before the owner has tested it and made it work.
- Tools are tested granularly: each does one thing, well, reliably, efficiently.
- The marketplace is a curated catalog of vetted tools, NOT an open LLM that generates anything any way. Quality over breadth.
- No raw connector is ever exposed without being vetted first.

## Layering

tools (primitives: scoped, tested, deterministic)
  -> agents (encapsulate one or more tools, a toolset + behavior, activatable/deactivatable per account)
  -> marketplace (distributes agents)
  -> triggers (note/voice "lance xxx" fires them)
  -> account link (decides which agents are active for this user)

Maps directly to the existing rig agent + Tool abstraction (an agent = a toolset + prompt, toggled).

## Architecture decisions (current, locked)

- 100% Rust now. Keep SQLite. Drop Better Auth / TanStack Start / Bun / Drizzle / Postgres for now (premature; reopen only when self-serve user login is actually needed).
- Premium gate becomes DB-backed: the already-present (currently dead) `devices.premium` column + a guarded admin endpoint (ADMIN_TOKEN env) to grant/revoke. Removes the per-user redeploy pain. Replaces the `PREMIUM_PUBKEYS` env allowlist.
- User self-serve login is deferred until users must log in themselves. At that point an account/identity layer is grafted on top; the entitlement flag/table stays. The JS-vs-Rust auth debate reopens only then.
- Constraints: zero hardcoding (every secret and URL in env, mirroring the backend `req_env`); proper `.gitignore` for any new code.

## References in the codebase

- RFC 0003: note -> calendar event = the proof of the note-as-action pattern.
- RFC 0008: MCP connectors via backend broker = the channel that lets the device pilot external tools.
- RFC 0009: user accounts, premium entitlements, admin = where account-based access is designed (Phase 1 = accounts + admin grant, no IAP).

## Pending op

- Backend #64 (callback GET + premium allowlist): MERGED to `main` (PR #1, 2026-06-21) and LIVE on api.flowflow.be (GET callback confirmed: probe returns 400, not 405). Remaining for #64 = commit the device-side changes in flowflow (connect_flow open-url+poll, oauth.rs removed) and device-test the connect flow end-to-end.

## Rust backend crate stack (researched 2026-06)

Build-ready libs for the 100%-Rust marketplace backend. Versions current as of 2026-06.

- Web: axum 0.8.9, tower-http 0.6.11, axum-extra (typed-header).
- DB: sqlx + SQLite. Keep 0.8.6, or move to 0.9.0 deliberately (SQLite breaking changes). WAL + busy_timeout(5s) + synchronous=Normal + foreign_keys. SQLite is fine at this scale; no Postgres.
- MCP: rmcp 1.7.0 (feature `transport-streamable-http-client-reqwest`, rustls; token via `auth_header`; `reinit_on_expired_session=true`; 404 = expired session). rig-core 0.38.2 with the `rmcp` feature (project is on 0.36; bump pulls rmcp ^1.7 and changes the integration API to `ToolServer`/`McpClientHandler`). PIN 0.38.2 - 0.39 is breaking (AgentRun refactor).
- Agents/tools: rig-core Tool trait + `#[rig_tool]` derive + schemars v1. Encapsulate a toggleable named set via `.tools(vec![...])` (simplest) or a shared `ToolServerHandle`. Multi-step via `.multi_turn(n)`. An agent has `.name()`/`.description()` = a named composable unit.
- Triggers (phrase -> tool): NO crate first - normalized `strip_prefix("lance ")` + `match` (deterministic, zero dep, YAGNI). aho-corasick only when the keyword set grows; fuzzy-aho-corasick if typo-tolerant. rig has no built-in router (LLM decides), so deterministic triggers need this small layer.
- Admin UI: maud 0.27 (inline `html!` macro, axum feature - least code for a few views) + htmx 2 via CDN (fragments returned on POST). askama 0.16 only if the panel grows large. Never rinja (merged into askama).
- Admin auth: single ADMIN_TOKEN bearer via a `FromRequestParts` guard (AdminAuth) or tower-http `ValidateRequestHeaderLayer::bearer`, constant-time compare (subtle). Cookie session (tower-sessions/axum-login) only if browser login UX is wanted.
- Jobs / automations / chained actions: apalis 1.0 + apalis-sqlite (durable queue: `run_after` deferred, retries+backoff, orphan recovery, same SQLite file) + apalis-workflow (`and_then` chains, DagFlow parallel) + apalis-cron for schedules (tokio-cron-scheduler can't persist to SQLite). External n8n: call its webhook via reqwest, wrapped as an apalis job for retries+durability. Zero extra infra.

Gotchas: rig 0.39 breaking (pin 0.38.2); rmcp version refs stale everywhere (use 1.7); sqlx 0.9 SQLite breaking changes (stay 0.8.6 unless deliberate); rig McpTool rejects audio content.
