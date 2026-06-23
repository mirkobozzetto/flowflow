# Phase 1 pre-build research (2026-06-19)

Two Exa-only research passes (Klavis Strata self-host; Dokploy compose deploy) before locking the build.
Source-of-truth files cited inline. The Klavis pass invalidates load-bearing Phase 1 assumptions - see "Blocking".

## Dokploy (clean - VERDICT: yes)

- Git-repo Docker Compose deploy, mixing `build:` (repo Dockerfile) + `image:` (pulled), is supported.
  Deploy cmd: `docker compose -p <proj> -f <file> --env-file <env> up -d --build`.
  (docs.dokploy.com/docs/core/docker-compose)
- One public HTTPS service via Domains tab (Let's Encrypt/Traefik); other services stay internal.
  (docs.dokploy.com/docs/core/docker-compose/domains)
- Compose rules: declare `dokploy-network` as `external: true` and attach it to EVERY service that must
  cross-talk (Dokploy only auto-attaches the domain'd service -> #3200 504s otherwise); use `expose` not
  `ports`; NO `container_name`; persistent volumes via `../files/...` (repo dir is wiped each deploy).
- Env: UI vars are written to a `.env` next to the compose file but NOT auto-injected -> use
  `env_file:` or `${VAR}`; build-time needs `ARG` + `build.args`.
- Install: `curl -sSL https://dokploy.com/install.sh | sh` (root Linux; ports 80/443/3000). Apache 2.0.

## Klavis "open-strata" + Google Sheets MCP (VERDICT: partially - design mismatch)

- `open-strata` = Python pkg `strata-mcp` (CLI `strata run --port 8080`), in `github.com/Klavis-AI/klavis`,
  Apache 2.0. No official Docker image (DIY container). Standalone, never calls Klavis.
- Strata exposes ONE endpoint (`/mcp`, `/sse`) but only 5 META-tools (progressive disclosure):
  `discover_server_actions`, `get_action_details`, `execute_action`, `search_documentation`,
  `handle_auth_failure`. The agent calls `execute_action` with a target action name + args - NOT the
  downstream tool directly. (open-strata/src/strata/server.py)
- Google Sheets MCP = `ghcr.io/klavis-ai/google-sheets-mcp-server:latest`, port 5000, `/mcp` + `/sse`.

### Four Phase-1 assumptions the source contradicts

1. **Strata does NOT do OAuth/token mgmt in standalone mode.** OAuth mode = `KLAVIS_API_KEY` which phones
   home to Klavis cloud (rejected: per-user cost + EU custody). Standalone = `SKIP_OAUTH=true` +
   operator supplies a minted Google access token via `AUTH_DATA` env (JSON) or base64 `x-auth-data`
   header. The Sheets server does NO refresh. (mcp_servers/google_sheets/server.py, _oauth_support/)
   -> The backend must own the Google OAuth app + refresh + token injection after all. The pivot premise
   "Strata removes the OAuth/token work" is FALSE for self-host standalone.

2. **Strata does NOT relay the client auth header downstream.** `handle_streamable_http`/`handle_sse`
   ignore incoming auth; downstream auth is static config `headers` or Strata's own host-side OAuth cache.
   -> The device's `auth_header` reaching Strata's SSE leg (P1.3 accept / finding 12) buys nothing for
   Sheets auth. The token-passthrough model is dead. (open-strata .../transport/http.py, auth_provider.py)

3. **`drive.file` is insufficient** to open an existing CRM sheet by ID (drive.file = only app-created/
   picker files). Writing needs `spreadsheets`; `list_spreadsheets` needs a Drive scope. Klavis docs ask
   full `auth/drive`. `spreadsheets` is a SENSITIVE scope -> Google verification/CASA at publish (fine
   for an unverified dogfood app under the test-user cap; a launch blocker later). The RFC chose
   `drive.file` specifically to dodge CASA - that dodge does not hold for an existing sheet.
   (klavis.ai/docs/.../google_sheets, server.py)

4. **No `add_row` tool.** Sheets MCP has `google_sheets_write_to_cell` (spreadsheet_id, column, row,
   value, sheet_name) - per cell. A row append = compute next empty row + N cell writes, or fork.
   P1.1 accept "a manual `add_row` works" references a tool that does not exist. (server.py)

### Research recommendation

For a single connector, Strata adds only progressive-tool-disclosure (context savings) and brings the
passthrough limitation. Simplest viable: the broker backend owns the Google OAuth app + refresh tokens
and injects a fresh access token into `google-sheets-mcp-server` via the `x-auth-data` header per request
(rotate without restart). Re-introduce Strata when there are 2+ connectors and tool-context matters.

Key source files (all github.com/Klavis-AI/klavis, main): `mcp_servers/google_sheets/server.py`,
`open-strata/src/strata/server.py`, `.../mcp_proxy/transport/http.py`, `.../mcp_proxy/auth_provider.py`,
`_oauth_support/README.md`.
