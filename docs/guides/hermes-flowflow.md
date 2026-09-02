# Use a FlowFlow space with Hermes Agent

FlowFlow can grant Hermes access to one shared space through MCP. The access is
scoped, revocable, and separate from the account and device credentials.

The reusable Hermes skill is published in
[`skills/flowflow-spaces`](../../skills/flowflow-spaces/).

## What Hermes can do

Hermes can:

- list the folders visible through one space token;
- list note metadata and read selected note bodies;
- search titles, bodies, and update dates by reading visible notes;
- list threads and read a thread's notes in canonical order;
- summarize or analyze explicitly selected content;
- create collaborative subfolders under writable parents;
- create and update its own notes with idempotent UUIDs;
- create its own threads from its own notes, and write notes inside threads;
- delete its own notes and threads after explicit confirmation;
- process and acknowledge changes with a server cursor.

Hermes cannot:

- access another space with the same token;
- write at the space root or in a non-writable folder;
- update or delete a human-authored note or thread;
- attach a human-authored note to a thread;
- recover a token after its one-time display;
- inspect attachments.

Thread order is the server creation time of the member notes. Hermes never
infers membership or order from titles or bodies.

## Requirements

- A FlowFlow account that owns the selected space.
- An active Premium entitlement for agent writes.
- A Hermes Agent installation that can reach `https://api.flowflow.be`.
- A collaborative folder in the selected space for write access.

## 1. Create the access in FlowFlow

1. Open FlowFlow on the iPhone or Mac.
2. Open the sidebar and find the space you own.
3. Open its three-dot menu.
4. Select **Hermes access**.
5. Keep the name `Hermes`, or choose another agent name.
6. Select **Read only** or **Read and write**.
7. Select **Create access**.
8. Copy the `mcps_` token immediately.

The token appears once. Do not paste it into Hermes chat, Telegram, a note, a
skill, source control, shell history, or logs.

Use one clearly named Hermes integration per space in this workflow. If one
already exists, rotate or revoke it instead of creating an indistinguishable
second integration.

## 2. Install the Hermes skill

Clone or update the FlowFlow repository on the Hermes host. From its root, run:

```sh
./scripts/install-flowflow-hermes-skill.sh
```

This copies the published skill to:

```text
~/.hermes/skills/productivity/flowflow-spaces/
```

An existing copy is moved to a timestamped backup first. Reload Hermes skills
or restart Hermes after installation.

### Keeping the skill and the backend in step

`space_info` returns `contract_version`. The skill states the version it was
written for. When the backend gains tools or fields, the version is bumped and
the skill tells the user to update instead of guessing:

```sh
cd flowflow && git pull && ./scripts/install-flowflow-hermes-skill.sh
```

Hermes can also track the skill itself once it is installed from the
repository URL, which enables `hermes skills check` and
`hermes skills update flowflow-spaces`:

```sh
hermes skills install https://raw.githubusercontent.com/mirkobozzetto/flowflow/main/skills/flowflow-spaces/SKILL.md --category productivity
```

Both paths install the same files. Use one of them consistently.

## 3. Store the token outside the configuration

Open `~/.hermes/.env` in a local editor on the Hermes host. Add one variable for
the space. Use an uppercase slug without spaces:

```dotenv
FLOWFLOW_TOKEN_PROJECTS=mcps_replace_with_the_one_time_token
```

Then restrict the file:

```sh
chmod 600 ~/.hermes/.env
```

Do not pass the real token as a command argument. Command arguments may remain
in shell history or process logs.

## 4. Configure the MCP server

Add this entry to `~/.hermes/config.yaml`:

```yaml
mcp_servers:
  flowflow_projects:
    url: "https://api.flowflow.be/v1/mcp-spaces"
    headers:
      Authorization: "Bearer ${FLOWFLOW_TOKEN_PROJECTS}"
    timeout: 30
```

Use one server name and environment variable per shared space:

```text
flowflow_<space-slug>
FLOWFLOW_TOKEN_<SPACE_SLUG>
```

Hermes expands `${VAR}` from its environment. The token stays out of the YAML
file and the published skill.

## 5. Verify the connection

Ask Hermes:

```text
Verify my FlowFlow projects space, list the visible folders, and do not write.
```

Hermes must call `space_info`, `pull_changes`, then `list_folders`.

Expected result:

- the reported space name matches the selected FlowFlow space;
- the scope is `read` or `read_write` as chosen;
- the token expiry is shown without exposing the token;
- every visible folder includes its name and write status;
- an empty space is reported as valid and empty, not broken.
Tokens expire within 365 days. Track the reported `expires_at` value and rotate
the token before that date.

Then ask for a first guided read:

```text
List the notes in <folder>. Show title, author, update time, and location.
Do not open or change anything until I choose one.
```

Hermes must present all plausible matches and ask for a choice when the target
is ambiguous.

## Search and analysis

The MCP server has no full-text search action. `list_notes` returns at most 100
notes. Continue with `after_seq=next_after_seq` until `next_after_seq` is
absent, then call `read_note` only for bodies needed by the search.

Examples:

```text
Find notes in Projects whose title contains pricing.
```

```text
Find notes in Projects whose body mentions annual billing.
```

```text
Show notes updated before 1 August 2026.
```

The exposed timestamp is `updated_at`. Hermes must not present it as the
creation date.

## Incremental changes

Call `pull_changes` without `since_seq` to resume from `last_ack_seq`. Process
every returned item. When `more` is true, retain `next_seq`, wait 30 seconds,
and continue with `pull_changes(since_seq=next_seq)`. Call `ack_changes` only
after every page and requested output succeeds.

For analysis:

```text
Summarize the note I selected. Separate the FlowFlow content from your
interpretation. Do not write the result back to FlowFlow.
```

For external research:

```text
Research this note's topic on the web. Keep the original note, your analysis,
and cited web sources in separate sections. Do not write to FlowFlow.
```

## Controlled writes

Before writing, confirm that the token scope is `read_write` and the target
folder has `writable: true`.

Create a folder:

```text
Create a collaborative subfolder named Daily reviews under Projects.
```

Create a note:

```text
Create a note named Review in Daily reviews with this exact content: ...
```

Hermes stores one non-secret agent namespace UUID in
`~/.hermes/state/flowflow-spaces/<server>.namespace`. UUID v5 object ids derive
from that namespace, the space, the parent or folder, the normalized name, and
the operation key. Keep the namespace across token rotation. Reset it only when
a replacement agent is created after revocation.

Hermes can update only a note returned with `own: true`. It must refuse to
modify a human-authored note.

Deletion requires an explicit confirmation after Hermes shows the exact note:

```text
Delete the Hermes note Review from Daily reviews.
```

A request to delete is not the final confirmation. Hermes must ask once more
before calling `delete_note`.

## Replies and Telegram

Hermes confirms work in the conversation where the request arrived. When that
conversation is on Telegram, the normal reply is also the Telegram
confirmation.

Hermes does not create a FlowFlow result note by default. Ask explicitly if the
result itself should become a note.

## Preferences and routines

Preferences describe folders, subjects, and output styles that matter to the
user. Hermes must restate them before applying them for the first time. They do
not change the general FlowFlow model or token permissions.

No monitoring or schedule is enabled by default. A routine needs an explicit
request and must define:

- the FlowFlow server and visible target;
- the schedule and timezone;
- whether it reads, writes, or both;
- the expected result and reply channel;
- the stable UUID strategy for repeated writes.

The routine must be inspectable, editable, suspendable, and removable. Removing
a routine does not revoke FlowFlow access.

Before creation, confirm the scheduler timezone. Set `HERMES_TIMEZONE` to the
requested IANA timezone and reload Hermes, or convert the schedule to the active
timezone. Do not create a routine with ambiguous daylight-saving behavior.

For Telegram, run `/sethome` in the intended chat or use an explicit
`telegram:<chat-id>` target. Then use the full lifecycle:

```sh
hermes cron create --name flowflow-review --skill flowflow-spaces --deliver telegram "0 9 * * *" "Use only flowflow_projects. <task>"
hermes cron list --all
hermes cron edit <job-id> --schedule "0 10 * * *"
hermes cron pause <job-id>
hermes cron resume <job-id>
hermes cron runs <job-id>
hermes cron remove <job-id>
```

Plain `hermes cron list` hides paused jobs.

## Rotation

1. Open **Hermes access** in the FlowFlow space menu.
2. Select **Regenerate** for the active agent.
3. Copy the new token immediately.
4. Before replacement, call `space_info` through the running old configuration.
5. Confirm that the previous token now returns HTTP 404.
6. Replace only the matching value in `~/.hermes/.env`.
7. Reload or restart Hermes so the process receives the new value.
8. Call `space_info` and confirm the expected scope and new expiry.

Rotation preserves the agent identity, note ownership, and namespace file.

## Revocation

1. Open **Hermes access** in the FlowFlow space menu.
2. Select **Revoke**.
3. Confirm that Hermes disappears from the space member list.
4. Confirm that the running old configuration now returns HTTP 404.
5. Remove the matching MCP server and environment value from the Hermes host.
6. Reset its namespace file before connecting a replacement agent.

Existing notes keep their agent authorship.

## Troubleshooting

| Result | Meaning | Next action |
|---|---|---|
| MCP server absent | Hermes has no space configuration. | Install the skill and MCP entry. |
| Timeout or connection error | Backend or network is unavailable. | Check `https://api.flowflow.be/healthz`. |
| HTTP 404 or `unauthorized` | Token is unknown, expired, rotated, or revoked. | Rotate or create access in FlowFlow. |
| `forbidden` | Scope or folder blocks the write. | Check `read_write` and `writable`. |
| Valid empty folder list | The space has no live folder. | Add or share a folder in FlowFlow. |
| Valid empty note list | The selected folder has no live note. | Choose another visible folder. |
| `rate limited` after `pull_changes` | Pulls are limited to one per 30 seconds. | Retain `next_seq`, wait 30 seconds, then continue. |
| `rate limited` after a write | More than 60 writes occurred in one minute. | Wait and retry with the same UUID. |
| `folder_cycle` | The move would create a cycle. | Choose a parent outside the subtree. |
| `folder_depth_exceeded` | The folder would exceed depth eight. | Choose a shallower parent. |
| `space_folder_limit` | The space reached 500 folders. | Remove unused folders as the owner. |
| `space_note_limit` | The space reached 5,000 notes. | Remove unused notes as their authors. |
| `note too large (max 64 KB)` | The note body is too large. | Shorten or split it with approval. |
| `title too long` | The title exceeds 200 characters. | Shorten the title. |
| `space_read_only` | Premium writes are unavailable. | Restore Premium or use read-only actions. |
| `not found` | The id is not live in this token's space. | List visible resources and choose again. |
| `operation failed` | The backend failed internally. | Retry once, then report the backend error. |

Diagnostics must never print the complete token or authorization header.
