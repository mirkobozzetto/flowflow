---
feature: "Hermes FlowFlow collaboration skill"
type: verification-bundle
status: blocked
---

# Verification bundle

## Passed

- The repository skill defines all nine deployed MCP space actions.
- Skill installation and timestamped backup passed in an isolated home.
- The final skill is installed on the Hermes VPS.
- A fresh Hermes process loaded the deployed namespace, rotation, cron, and
  Telegram rules.
- Live `space_info`, `pull_changes`, `list_folders`, `list_notes`, and
  `read_note` calls passed without mutation.
- The live connection reported `read_write`, one folder, and two notes.
- Evidence contains no complete token, note body, or object identifier.
- Markdown structure, line length, and forbidden-dash checks passed.

## Not executed against live state

- `ack_changes` because it advances the server cursor.
- Folder and note writes because they mutate the real space.
- Note deletion because it requires an exact user confirmation.
- Telegram delivery and cron lifecycle because they require a confirmed chat,
  schedule, and timezone.
- Token rotation and revocation because they invalidate live credentials.
- iPhone pull verification because it requires the physical device.

## Blocker

Ordered thread reading cannot pass. The current MCP contract exposes no thread
identifier, title, membership, ordering field, or dedicated thread action.
The skill reports this limitation and never infers thread topology.
