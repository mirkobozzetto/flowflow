# Issue 101 - Block A trace

User approved grouped block A (2a + 2b), sequential execution with tested commits
and pushes on dev. No intermediate approval required. B/C design and native
consent decisions, provider configuration and deployment remain separate gates.

## Completed

- 2a: shared chat/note menu queries the existing backend connector list on mount.
  Google account status is not represented as tool/resource readiness.
  Loading, disconnected, unavailable and connected labels; check only on confirmed
  connection. Four library tests passed, including FR/EN label coverage.
  Command: `cargo test --lib ui::chat::connector_status::tests`.
  No device/browser or live account check performed.

## Block A implementation complete

- 2b: generic chat now uses a pool of connector-specific peers, filtered per
  owner, and supplies the same peer map to approval execution. Pool lifetime
  encloses the prompt. Closed/absent approval channels mount no connectors.
- A failed service, ambiguous names/prefixes or missing advertised tools falls
  back to native-only with a visible warning in the answer. The fallback does
  not retry through the weaker legacy alias.
- Backend route delivered in marketplace commit 0aba6e8:
  `/v1/chat/connectors/{slug}/mcp`; premium/device and connector access checks,
  mandatory active manifest, rejected batches/malformed calls. Existing agent
  route still requires x-agent-id; legacy route unchanged.
- Checks: 14 chat_surface tests, 10 agent_builder tests, 21 contract_hook tests,
  7 governance_approval tests passed. Backend: 3 proxy HTTP tests passed.
  Four 2a UI-state tests passed separately. No live provider/device/LLM test.
- First 2b run had 9 chat tests; final run added 5 ownership/routing regressions.
  Compiler emitted existing block future-compatibility and large unwind-table
  warnings, not failures. The test binaries actually ran.
- Source review and GitNexus scope agree on chat/MCP changes; graph process
  discovery is incomplete and its cached freshness hint disagrees with the
  current local index metadata. Do not interpret missing edges as safety proof.

## Impact and release constraints

- 2b: inspected chat surface, MCP pool and backend proxy. GitNexus impact of
  prompt_chat_agent is HIGH: two direct callers, three process groups (chat,
  notes, RAG). Preserve per-tool routing, approval channel and session lifetime.
- Backend connector slug route requires x-agent-id. Generic chat must not supply
  a fake agent or weaken that route. Legacy /v1/mcp is Google-only. Determine a
  separate authenticated generic route with explicit connector authorization.

## Preserve

Unrelated docs/proposals/0010-account-home-redesign/ remains untracked and untouched.
No main merge, live OAuth/service change, deployment or native consent change.

## Release follow-up requested by user

Before deployment, locate the `flowflow` connector linked to RMS and its
associated documentation, skill and MCP integration. Access/location are not yet
confirmed. Review and update affected contracts, examples and skill guidance for
the shipped changes; verify consistency before release. Do not assume current
MCP access includes RMS. This checklist item does not authorize deployment or
external service changes and does not pause block A.

## Delivery gate

After committing block A, open PR(s) from dev to main and inspect available CI.
Do not merge or deploy. Wait for the user's signal before block B, with binding
compatibility/migration and native-write consent decisions still required.
Deploy the new backend route before releasing this client; against an old
backend, connector setup fails visibly and chat is native-only. The old client
can continue using the unchanged legacy backend alias.

Manual release checks still needed: chat, note actions and RAG with a real test
account; approve/reject/cancel an external write; disconnect/reconnect; one failed
connector; return from settings and reopen both menus. No device installation
or successful live service run is claimed by unit/integration tests.
