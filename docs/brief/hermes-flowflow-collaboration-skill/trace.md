---
feature: "Hermes FlowFlow collaboration skill"
type: ship-trace
status: blocked
---

# Delivery trace

## Decisions

- The reusable skill lives in the FlowFlow repository under
  `skills/flowflow-spaces/`.
- The installer copies it into the standard Hermes skill directory.
- The public guide lives in `docs/guides/hermes-flowflow.md`.
- Secrets remain in `~/.hermes/.env`, never in YAML, chat, notes, or skills.
- Search uses MCP pagination and selective note reads because no server-side
  full-text search action exists.
- Date filtering uses `updated_at` and labels it as update time.
- A durable agent namespace makes caller UUIDs idempotent without collisions.
- Thread structure is reported as unavailable instead of being inferred.

## Verified delivery

- The skill and references cover all nine deployed MCP actions.
- The installer passed syntax and two-install backup checks.
- The final skill was installed on the Hermes VPS with a timestamped backup.
- A fresh Hermes process recalled namespace, rotation, cron, and Telegram rules.
- Live read-only MCP calls passed for `space_info`, `pull_changes`,
  `list_folders`, `list_notes`, and `read_note`.
- The live scope was `read_write`; one folder and two notes were visible.
- No live write, cursor acknowledgement, routine, rotation, or revocation ran.
- No complete token, note body, or object identifier entered the evidence.

## Blocked acceptance

US5 and the thread scenario are blocked because the MCP model exposes no thread
fields or actions. Full 10/10 acceptance also needs manual mutation, Telegram,
routine, token lifecycle, and iPhone scenarios. Those require explicit live
state actions and user-selected targets.
