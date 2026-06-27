# Ship contract - RFC 0014 Phase P0 (model modernization)

Source: `docs/rfcs/0014-platform-web-app-accounts-roles-agent-management/RFC.md` (status: Accepted)
Scope: P0 only = tasks T01, T02, T03 (device repo `flowflow`). Engine: solo.

## Definition of done (one row per acceptance criterion)

| # | Task | Criterion | Verified by |
|---|------|-----------|-------------|
| C1 | T01 | `CHAT_MODEL = "gpt-5.4-mini"`, `CHEAP_MODEL = "gpt-5.4-nano"` added, `EMBEDDING_MODEL` unchanged; build green | constants.rs + `make check` |
| C2 | T01 | `gpt-5.4-nano` actually used (tag + title generation), not a dead const | llm.rs `generate_tags`/`generate_title` |
| C3 | T02 | An agent's manifest `model` selects the chat model at runtime; default falls back to mini | `resolve_chat_model` + chain wiring |
| C4 | T02 | A test asserts the override | `tests/llm_test.rs::test_resolve_chat_model_*` |
| C5 | T03 | No `gpt-4o` left in the device repo (constants, fixtures, tests) | grep + re-signed fixture |
| C6 | T03 | Each pinned id returns 200 from the live OpenAI API | curl `/v1/models/{id}` |

## Out of scope (never build in P0)

- P1-P4: web accounts, roles/RBAC, access-requests, connector/agent split, chain/canvas builder, front refonte.
- Anthropic cheap-model split (`claude-haiku-4-5`): deferred; Anthropic keeps `claude-sonnet-4-6`.
- Per-agent `temperature` wiring (still read, still unused): not in T01-T03.
- Publish-time model allowlist + provider/model coherence (M4 / TA9): backend, later phase.
- mkt (`marketplace-flowflow`) seed manifest re-sign + redeploy (T03 mkt half): deferred, see trace. Neutralized
  on-device by the `resolve_chat_model` legacy map, so prod serving `gpt-4o` runs as `gpt-5.4-mini`.

## Edit scope (authorized files)

- `src/application/constants.rs`
- `src/infrastructure/llm.rs`
- `src/application/chain.rs`
- `src/application/connector_module.rs`
- `tests/agent_builder_test.rs`
- `tests/agent_manifest_test.rs` (`gen_fixture` signer via env)
- `tests/llm_test.rs`
