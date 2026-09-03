# FlowFlow workflows

## Discover visible content

1. Call `space_info`.
2. Call `list_folders`.
3. Describe only folders returned by the server.
4. If no folders exist, report a valid but empty space.

Show folder name, hierarchy, mode, and whether it is writable.

## Find notes

1. Resolve an optional folder with `list_folders`.
2. Page through `list_notes` with `after_seq` until
   `next_after_seq` is absent.
3. Filter titles case-insensitively when the title can answer the query.
4. For content search, call `read_note` on candidates and compare their bodies.
5. For date requests, filter by `updated_at`. Say that this is update time.
6. Present matches with title, author, update time, folder, and id.
7. If several candidates match, ask the user to choose before acting.
8. If none match, report no result. Never invent a likely note.

Content search is client-side and may require reading every visible note.

## Read and summarize

1. Resolve exactly one note from live metadata.
2. Call `read_note`.
3. Quote or summarize only the returned body.
4. Label deductions as interpretation, not FlowFlow content.
5. Do not write the answer back to FlowFlow unless explicitly asked.

For web research, start only after an explicit request. Keep FlowFlow content,
interpretation, and external sources in separate response sections. Cite source
URLs.

## Threads

A thread starts from exactly one existing note. That note is the root and may
be human-authored. Later notes continue the root. A note belongs to at most one
thread. `read_thread` returns members oldest first.

### List and read a thread

1. Call `list_threads`, optionally with a `folder_id`.
2. If several threads match, ask the user to choose.
3. Call `read_thread` on the chosen id.
4. Call `read_note` only for member bodies needed by the request.
5. Present the thread title separately from the note titles.
6. Treat a zero-member thread as legacy data and do not create another one.

### Continue from a selected note

1. Resolve exactly one existing note and its writable folder.
2. If the note has a `thread_id`, call `read_thread` and use that thread.
3. If it has no `thread_id`, derive a stable thread UUID v5:
   `thread:<space_id>:<folder_id>:<root_id>`.
4. Call `create_thread` with that id and `note_ids: [root_id]`.
5. The root may have `own: false`. Linking it must not change its title,
   body, or author.
6. Derive a stable continuation note UUID using the normal note workflow.
7. Call `put_note` with the thread id and the complete continuation body.
8. Call `read_thread`. Require the root first and the new note last before
   reporting success.

Never append a later human-authored note. Later notes must be created by this
agent through `put_note` with the existing `thread_id`.

### Rename or delete a thread

Only threads with `own: true` can be changed. Rename by replaying
`create_thread` with the same id, the new title, and no `note_ids`. Delete
with `delete_thread` after explicit confirmation. Member notes survive
without a thread.
## Stable agent namespace

Before the first write through a server:

1. Use `~/.hermes/state/flowflow-spaces/<server>.namespace`.
2. If absent, create it with one random UUID v4 and store only that UUID.
3. Keep it across retries, sessions, and token rotation for the same agent.
4. Reset it only after revocation and creation of a replacement agent.

This namespace is not a secret. It prevents a replacement agent from deriving
ids still owned by the revoked agent.

## Create a collaborative folder

1. Confirm the user explicitly requested creation.
2. Resolve one parent whose `writable` value is `true`.
3. Ask for clarification if the parent is ambiguous.
4. Normalize the requested name by trimming and collapsing whitespace.
5. Derive a UUID v5 with the stored agent namespace and this name:
   `folder:<space_id>:<parent_id>:<normalized-name>`.
6. Call `create_folder` with that UUID, parent id, and requested name.
7. Reuse the same UUID on every retry and in every later session.
8. Report the returned id and whether `created` is true or false.

The backend always creates the folder in collaborative mode.

## Create or update a note

1. Confirm the user explicitly requested a write.
2. Resolve one writable folder.
3. For creation, normalize the title and choose an operation key.
4. Default the operation key to the normalized title.
5. Derive a UUID v5 with the stored agent namespace and this name:
   `note:<space_id>:<folder_id>:<normalized-title>:<operation-key>`.
6. For update, select a note with `own: true` and reuse its id.
7. Call `put_note` with the id, folder id, title, and complete body.
8. Reuse the same id on retries and in every later session.
9. Require a distinct operation key for separate notes with the same title.
10. Report the returned id and whether the operation created or updated it.

Refuse updates to notes with `own: false`.

## Delete a note

1. Resolve exactly one note.
2. Verify `own: true`.
3. Show its title, folder, update time, and id.
4. Ask for explicit deletion confirmation.
5. Call `delete_note` only after confirmation.
6. Report success in the active conversation.

Never delete a human-authored note.

## Process changes

1. Call `pull_changes` without `since_seq` to resume from `last_ack_seq`.
2. Process all returned folder and note metadata.
3. Read only bodies required by the requested work.
4. If `more` is true, retain `next_seq` and wait at least 30 seconds.
5. Continue with `pull_changes(since_seq=next_seq)` after that delay.
6. Complete every requested output.
7. Call `ack_changes` with the highest fully processed sequence.

Do not acknowledge a partial or failed batch.
