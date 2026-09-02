# Hermes and FlowFlow manual test plan

Do not accept this delivery until all ten scenarios pass in one run.

## Prerequisites

- A FlowFlow Premium owner account.
- One real test space and one unrelated control space.
- A collaborative parent folder.
- Two notes with similar titles and different bodies.
- Notes with known `updated_at` values.
- One human-authored note that must remain unchanged.
- One thread containing at least three ordered notes.
- A clean Hermes skill directory on the VPS.
- An active Telegram conversation with that Hermes instance.
- The current FlowFlow build installed on one iPhone.

Never paste the complete `mcps_` token into this document, chat, screenshots,
logs, or command history.

## Result record

For each scenario, record `PASS` or `FAIL`, its timestamp, and redacted
evidence.
A blocked scenario counts as `FAIL` for the 10/10 acceptance target.

## Scenario 1 - Install and connect

1. Create read and write Hermes access from the test space menu.
2. Copy the one-time token directly to `~/.hermes/.env`.
3. Install the repository skill with the installer.
4. Add the MCP server entry and reload Hermes.
5. Ask Hermes to verify the space without writing.

Expected:

- `space_info` reports the exact test space and `read_write` scope;
- `list_folders` reports only folders from that space;
- no output contains the complete token.

## Scenario 2 - Diagnose known states

1. Verify the valid connection.
2. Query an empty folder.
3. Query a nonexistent id.
4. Replace the environment value with a revoked token.
5. Reload or restart Hermes and verify that `space_info` returns HTTP 404.
6. Restore the valid token.
7. Reload or restart Hermes and verify the valid connection again.

Expected:

- the empty folder is not described as a backend failure;
- the missing id is not described as a permission failure;
- the revoked token is identified as invalid, expired, rotated, or revoked;
- every failure includes one safe next action;
- no secret is printed.

## Scenario 3 - Discover and select

1. Ask Hermes to list visible folders and notes.
2. Ask for a title shared by two notes.
3. Do not choose a note yet.

Expected:

- results include title, author, `updated_at`, folder, and id;
- Hermes presents both candidates;
- Hermes asks which note to use;
- Hermes performs no write.

## Scenario 4 - Search titles, bodies, and dates

1. Search by one title word.
2. Search by one body word absent from all titles.
3. Filter notes before and after a known `updated_at` date.
4. Search for a term that does not exist.

Expected:

- title and body searches return the prepared notes;
- date results call the timestamp an update time;
- the empty search invents no result;
- every result belongs to the authorized space.

## Scenario 5 - Read a thread in order

Current status: blocked by the MCP contract.

The MCP server does not expose thread ids, thread titles, membership, or order.
This scenario cannot pass until those fields or dedicated thread tools are
available. The skill must state the limitation and must not guess.

After the contract is extended:

1. Ask Hermes to list threads in the prepared folder.
2. Select the prepared thread.
3. Ask for its notes in order and a summary.

Expected:

- thread title and note titles are distinct;
- all three notes appear in their real order;
- an incomplete or empty thread is reported explicitly;
- no note from the control space appears.

## Scenario 6 - Summarize and research

1. Select one note explicitly.
2. Ask for a summary and interpretation without writing.
3. Ask for related web research with cited sources.

Expected:

- FlowFlow content, interpretation, and web sources are separate;
- source URLs are usable;
- no FlowFlow object is created or changed.

## Scenario 7 - Create and update

1. Ask Hermes to create a collaborative subfolder under the writable parent.
2. Ask it to create one note in that folder.
3. Replay the same logical creation request.
4. Ask it to update that note.
5. Pull changes on the iPhone.

Expected:

- the folder is created under the requested parent;
- the replay reuses each stable UUID;
- exactly one folder and one note exist;
- the note contains the updated body once on the iPhone;
- Hermes confirms in the active conversation.

## Scenario 8 - Protect human content and delete agent content

1. Ask Hermes to update the prepared human note.
2. Ask Hermes to delete the prepared human note.
3. Ask Hermes to delete its own test note.
4. Decline the first confirmation, then repeat and confirm.

Expected:

- both human-note mutations are refused;
- declining confirmation causes no deletion;
- explicit confirmation deletes only the Hermes note;
- the refusal explains the permitted correction.

## Scenario 9 - Exercise cursors and all nine actions

Exercise and record one expected result for:

1. `space_info`;
2. `list_folders`;
3. `list_notes`;
4. `read_note`;
5. `pull_changes`;
6. `ack_changes` after complete processing;
7. `put_note` with a stable UUID;
8. `create_folder` with a stable UUID;
9. `delete_note` for an agent-owned note after confirmation.

Expected:

- all nine actions return their documented result;
- `ack_changes` never moves the cursor backward;
- no partial batch is acknowledged;
- all writes remain inside the authorized space.

## Scenario 10 - Confirm preferences, routines, channels, and access lifecycle

1. Define one folder or output preference and ask Hermes to restate it.
2. Confirm it, change it, then stop applying it.
3. Verify `hermes cron list --all` contains no unrequested FlowFlow routine.
4. Confirm the scheduler timezone and set Telegram home with `/sethome`.
5. Create a confirmed test routine attached to `flowflow-spaces`.
6. List, edit, pause, list with `--all`, resume, run, and inspect its runs.
7. Remove the routine and verify it is absent from `cron list --all`.
8. Send a read request from the active Telegram conversation.
9. Send a write request and verify no result note is created by default.
10. Explicitly request a FlowFlow result note.
11. Regenerate the token while Hermes still holds the previous value.
12. Verify that `space_info` now returns HTTP 404.
13. Replace the environment value, reload Hermes, and verify the new token.
14. Revoke access, reload Hermes, and verify HTTP 404 again.

Expected:

- preferences change without changing the skill or token scope;
- no routine exists by default and the full lifecycle is observable;
- the routine fires in the confirmed timezone and reaches the intended chat;
- Telegram receives the normal confirmations;
- only the explicitly requested result note is created;
- the new token works after rotation;
- the previous and revoked tokens return HTTP 404;
- the namespace survives rotation and resets for a replacement agent;
- existing agent-authored objects keep their authorship;
- no output reveals either token.

## Acceptance

Accept only when:

- every scenario is `PASS` in one complete run;
- all nine MCP actions have redacted evidence;
- the iPhone shows one copy of every idempotent write;
- no unauthorized read, implicit write, or human-note mutation occurred;
- Scenario 5 passed through real thread data, not inference.
