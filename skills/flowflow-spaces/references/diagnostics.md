# FlowFlow diagnostics

Never print a token, authorization header, environment value, or unredacted
configuration. Refer to the token only as `mcps_...`.

## Diagnostic order

1. Confirm the `flowflow_<slug>` server exists in Hermes.
2. Call `space_info`.
3. Call `pull_changes`, respecting its 30-second throttle.
4. If both succeed, call `list_folders`.
5. Only then decide whether content is empty, absent, or inaccessible.

## Outcomes

| Observation | Meaning | Next action |
|---|---|---|
| Server missing | Hermes is not configured for the space. | Follow the installation guide. |
| Connection or timeout error | Backend or network is unavailable. | Check `/healthz` from the Hermes host. |
| `unauthorized` or HTTP 404 | Token is invalid, expired, rotated, or revoked. | Create or rotate access in FlowFlow. |
| `forbidden` | The token or folder lacks the requested write permission. | Check scope and folder `writable`. |
| `not found` after valid `space_info` | The id is absent from this space. | List visible resources and ask for a target. |
| Empty folder list | The connected space has no live folder. | Create or share a folder in FlowFlow. |
| Empty note list | The selected scope contains no live note. | Check another visible folder or stop. |
| `rate limited` after `pull_changes` | Pulls are limited to one per 30 seconds. | Retain `next_seq`, wait 30 seconds, then continue. |
| `rate limited` after a write | The 60 writes-per-minute limit was exceeded. | Wait, then retry with the same UUID. |
| `folder_cycle` | The move would create a folder cycle. | Choose a parent outside the folder subtree. |
| `folder_depth_exceeded` | The folder would exceed depth eight. | Choose a shallower writable parent. |
| `space_folder_limit` | The space reached 500 folders. | Remove unused folders as the owner. |
| `space_note_limit` | The space reached 5,000 notes. | Remove unused notes as their authors. |
| `note too large (max 64 KB)` | The body exceeds the backend limit. | Shorten or split it with approval. |
| `title too long` | The note title exceeds 200 characters. | Shorten the title. |
| `space_read_only` | Premium writes are unavailable. | Restore Premium or use read-only actions. |
| `operation failed` | The backend returned an internal error. | Retry once, then report backend failure. |

A valid empty result is not a backend failure. A missing object is not proof of
missing permission. A permission error is not proof that the object exists.

Every failure response must include:

- the operation that failed;
- the redacted error class;
- the space or visible target involved;
- the next safe action.
