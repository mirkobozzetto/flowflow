# ship contract - RFC 0008 (scoped: T00a, T00b, T02b)

Source: `docs/rfcs/0008-mcp-connectors-oauth-broker-backend/RFC.md` (status: Accepted).
Scope (user-locked): the three gating spikes only. Stop after each; Mirko validates before the next.
Engine: solo. These are throwaway exploratory spikes, not app features.

Spike code location: `~/code/flowflow-spikes/<task>/` (outside the app repo, throwaway).

## Definition of done

### T00a - rig rmcp builder API + host connect; pin rmcp version
Depends on: none. Effort: M.
- [ ] A host binary connects via rig to a public Streamable-HTTP MCP server and lists its tools.
- [ ] The agent-builder method that compiles is identified (`rmcp_tools` / `tool_server_handle` / `McpTool::from_mcp_server`).
- [ ] `cargo tree -i rmcp` shows exactly ONE rmcp version (no two-version collision - finding 17).
Resolves: OQ#1, finding 17.

### T00b - aarch64-apple-ios link + on-device connect
Depends on: T00a. Effort: M.
- [ ] `rig[rmcp]+rmcp` LINKS for `aarch64-apple-ios` (not just `cargo build` - a real device link).
- [ ] An on-device run connects to an MCP server and lists tools.
- [ ] The device sends `auth_header` on its OWN SSE leg (findings 12, 14).
Resolves: findings 12, 14; the iOS-cross-compile risk gate.

### T02b - token-injection spike vs self-hosted github-mcp-server
Depends on: T01 (backend bootstrap) per the DAG, but runnable as a minimal LOCAL proxy for the spike. Effort: S.
- [ ] A known-good GitHub token yields a successful MCP tool call through a minimal proxy to `github-mcp-server --http`.
Resolves: finding 6. Gates T03b/T04.

## Out of scope (not built here)
Everything else in §10 (T01, T02, T03a/b, T04a/b/c, T05-T13, T10). No backend repo, no app edits, no monetization.

## Guardrails
- No edits to the Accepted RFC.md (progress lives in trace.md).
- No edits to the flowflow app build (spikes are standalone crates).
- ship does not run the device/deploy steps; those are Mirko's on-device validation gates.
