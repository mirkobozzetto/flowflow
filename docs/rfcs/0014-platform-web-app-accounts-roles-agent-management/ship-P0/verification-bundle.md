# Verification bundle - RFC 0014 P0

Stack: rust / cargo. Claude already ran the SAFE set (all green); re-run any below to confirm.

## SAFE checks (already run, green)

| Command | Validates | Contract |
|---------|-----------|----------|
| `make check` | fmt + clippy `--features mobile`, 0 warnings | C1 |
| `cargo test --test llm_test` | `resolve_chat_model` override / default / legacy map (33 tests) | C3, C4 |
| `cargo test --test agent_manifest_test` | re-signed fixture verifies vs the pinned key (9 tests) | C5 |
| `cargo test --test agent_builder_test --test connector_module_test --test installed_agent_repo_test` | manifest build + install path, model = `gpt-5.4-mini` (28 tests) | C5 |
| `grep -rn "gpt-4o" --include="*.rs" .` | zero hits in the device repo | C5 |
| `curl -s -o /dev/null -w "%{http_code}" https://api.openai.com/v1/models/gpt-5.4-mini -H "Authorization: Bearer $OPENAI_API_KEY"` | id is live (200) | C6 |
| (same for `gpt-5.4-nano`) | id is live (200) | C6 |

## Device tap test (Mirko)

The only thing left to validate by hand. P0 is pure model selection, so the check is "the agent still runs, now on gpt-5.4-mini".

1. Open FlowFlow on the iPhone.
2. Settings -> Connections -> run the CRM / Sheets agent (the "synchro-clients" sync, e.g. "list my spreadsheets / what's in the leads sheet").
3. Expect: the chain runs end to end and answers from the live sheet, same as before. No model error, no "agent revoked", no verify failure.

If it answers correctly, P0 is confirmed on device. Tags/titles on new notes now run on `gpt-5.4-nano` (cheaper, same behavior).

## Deferred (not part of this device P0)

mkt seed manifest + redeploy (T03 backend half). Neutralized on-device by the legacy map. To finish later:

Update the backend seed model to gpt-5.4-mini, regenerate its fixture digest/signature test vectors, redeploy, re-publish the agent row.
`/ship RFC 0014 P0 mkt half in marketplace-flowflow: src/db.rs agent_config + AGENT_CRM_SYNC_MANIFEST -> gpt-5.4-mini, regen FIXTURE_DIGEST/FIXTURE_SIGNATURE in tests/agent_packaging.rs + integration.rs asserts, then redeploy + re-publish agent-crm-sync`
