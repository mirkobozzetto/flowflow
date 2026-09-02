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

A thread is a titled group of notes. A note belongs to at most one thread.
Member order is the server creation time, oldest first, as returned by
`read_thread`. Never reorder or infer membership from titles or bodies.

### List and read a thread

1. Call `list_threads`, optionally with a `folder_id`, to see live threads
   with their title, author, update time, and `note_count`.
2. If several threads match the request, ask the user to choose.
3. Call `read_thread` on the chosen id. It returns the thread and its member
   note metadata in canonical order.
4. Call `read_note` only on the members whose bodies are needed.
5. Present the thread title separately from the note titles.
6. Report an empty thread (`note_count` 0) explicitly.

### Create a thread from a note

1. Confirm the user explicitly requested the thread.
2. Resolve one writable folder and the notes to attach. Only notes with
   `own: true` can be attached; refuse human-authored notes and explain.
3. Normalize the title and derive a UUID v5 with the stored agent namespace:
   `thread:<space_id>:<folder_id>:<normalized-title>:<operation-key>`.
4. Call `create_thread` with that id, the folder id, the title, and
   `note_ids`. Reuse the same id on every retry.
5. Report the returned id and whether `created` is true or false.

### Write a note inside a thread

Follow "Create or update a note" and pass the thread id as `thread_id`. On an
update, `put_note` replaces the whole note: omitting `thread_id` detaches it.

### Rename or delete a thread

Only threads with `own: true` can be changed. Rename by replaying
`create_thread` with the same id and the new title. Delete with
`delete_thread` after explicit confirmation; member notes survive detached.

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
