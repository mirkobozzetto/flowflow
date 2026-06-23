# Marketplace / connectors roadmap

Companion to [marketplace-agents.md](marketplace-agents.md). Tracks what shipped and what is left for
the platform-driven connector model. English per repo convention.

## Done (2026-06-21)

- **OAuth #64** verified end-to-end on device: tap Connect -> system Safari -> Google consent ->
  backend callback -> app shows connected. Device-side `connect_flow` = open-url + backend-poll, no
  custom scheme.
- **DB-backed premium gate + admin endpoint** (backend `marketplace-flowflow` PR #2, merged + live on
  api.flowflow.be). `devices.premium` flag replaces the `PREMIUM_PUBKEYS` env allowlist;
  `POST /v1/admin/premium` + `GET /v1/admin/devices` guarded by `ADMIN_TOKEN`, with an
  IP-independent brute-force throttle. No redeploy to grant/revoke.
- **Connections settings**: copyable Device ID (the device's base64 pubkey), bare-host backend URL
  (`api.flowflow.be` -> https auto).
- **Note-as-action**: a note runs through the agent + connected tools (generalizes RFC 0003 note ->
  calendar). Terse one-line result + resource link, cached per note so reopening shows the outcome
  and the button becomes "Run again".

## Todo

### Phase 2 - platform-driven catalog (drop hardcoding)
- [ ] Move the connector/agent catalog to DB, driven per account/device entitlements.
- [ ] `/v1/connectors` returns the granted catalog, not a hardcoded `google` row (`oauth::list`).
- [ ] Redo the Connections UI (current connector card is cramped/unpolished).

### Note-as-action polish
- [ ] Detect whether a note is actionable (LLM detect, or a `lance xxx` trigger phrase) so the run
      button is not shown on every note.
- [ ] Render the result as a proper card (created-check + prominent clickable link), not raw markdown.
- [ ] Same clickable-link + confirmation treatment in chat.

### Accounts (RFC 0009)
- [ ] Account model: one user, up to 3 devices, premium owned by the account (not per key).
- [ ] Gate resolves `pubkey -> account -> premium`.

### Fronts
- [ ] Admin web panel (maud + htmx): grant/revoke/list devices visually instead of curl.
- [ ] Marketplace web front (login, catalog, account link). Phase 3, deferred until self-serve login
      is actually needed.

### Later / nice-to-have
- [ ] Note-action latency: cache the MCP connection between calls.
- [ ] Remove the backend test device (`GX9r...`).
