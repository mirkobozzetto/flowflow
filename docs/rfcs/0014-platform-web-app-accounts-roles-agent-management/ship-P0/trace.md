---
artifact: docs/rfcs/0014-platform-web-app-accounts-roles-agent-management/RFC.md
artifact_kind: rfc
phase: P0
engine_tier: solo
final_status: shipped
date: 2026-06-26
---

# Trace ledger - RFC 0014 P0

| Task | Status | Files | Summary |
|------|--------|-------|---------|
| T01 | done | `src/application/constants.rs`, `src/infrastructure/llm.rs` | `CHAT_MODEL` -> `gpt-5.4-mini`; added `CHEAP_MODEL = gpt-5.4-nano`; routed `generate_tags` + `generate_title` to the cheap model via a new `chat_with_model`. Embeddings + Anthropic unchanged. |
| T02 | done | `src/infrastructure/llm.rs`, `src/application/chain.rs`, `src/application/connector_module.rs`, `tests/llm_test.rs` | Added `resolve_chat_model` (manifest model wins; blank or legacy `gpt-4o*` -> default). Threaded the model into `run_agent_over_tools` (OpenAI path). Refactored `run_chain` to take `&BuiltAgent` (resolves the model + clones gov/conn inside; removes the 8-arg clippy smell). 3 override/default/legacy tests. |
| T03 | done (device) | `tests/agent_builder_test.rs`, `src/application/connector_module.rs`, `tests/agent_manifest_test.rs` | Removed every `gpt-4o` from the device repo. Unsigned test manifest -> `gpt-5.4-mini`. Signed `FIXTURE_PACKAGE` -> `gpt-5.4-mini`, re-signed with the production seed; `gen_fixture` now takes the seed via `FIXTURE_SIGN_SEED` (never committed). Live API: `gpt-5.4-mini` 200, `gpt-5.4-nano` 200. |

## Re-sign provenance (T03)

- Pinned `ADMIN_PUBKEY` = `ed25519:7xalCAYJE6u/ydk7ruheDUnO+6YAiK8X2Y6If2CCXoE=` (production key).
- Verified: the production signing seed derives exactly to that pinned public key.
- New fixture `content_digest` = `sha256:98cbcbe81a0b5e614944f1c6a1682ee1b75ff425752e46ebfc23c75fcfac3557`.
- Regenerated via the project's own `gen_fixture` test (identical canonicalization), the seed passed through the
  environment, so no private key lands in the repo. `fixture_verifies_against_pinned_key` re-verifies the new signature.

## Checkpoints

- No DB writes, no deploys performed (both stay user-gated).
- mkt half of T03 deferred on purpose: `marketplace-flowflow` serves the manifest from a DB row seeded with
  `INSERT OR IGNORE`, so a source const change is inert on the already-seeded prod DB until a re-publish (a DB write),
  and its packaging tests assert a hardcoded digest/signature that would need regenerating. The on-device
  `resolve_chat_model` legacy map already maps any served `gpt-4o` to `gpt-5.4-mini`, so this is non-blocking.

## Verification

- `make check` (fmt + clippy `--features mobile`): clean, 0 warnings.
- Host tests: agent_manifest_test 9, agent_builder_test 4 (+1 ignored), connector_module_test 18,
  installed_agent_repo_test 6, llm_test 33 - all green (70).
- iOS build + install (`make all`): see handoff.
- Device tap test (Mirko): see verification-bundle.md.
