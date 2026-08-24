---
artifact: "docs/proposals/0002-collaborative-shared-folders/PROPOSAL.md"
artifact_kind: "propose"
run_id: "-T07-T16"
repo: "/Users/mirkobozzetto/code/flowflow"
work_branch: "feat/spaces-app"
base_branch: "dev"
stack: "rust / cargo"
locked: "2026-08-24"
---

# Contract: spaces app side (proposal 0002, T07-T16)

Definition of done for this run. One row per acceptance item. A requirement
change gets a NEW row, never a silent rewrite.

## Resolved open questions (answered before the run)

| Q | Answer |
|---|--------|
| Q4 | A space voice note travels as transcription only. No audio file crosses the backend. |
| Q6 | A note whose author leaves without asking for removal stays; the author is greyed, like the `gone` provenance state. |

Q1 (owner erasing a revoked member's notes) and Q5 (per-space moderation)
block no task in this run.

## Acceptance criteria

| # | Task | Criterion |
|---|------|-----------|
| C1 | T07 | Migration V26 applies on a V25 database: `spaces`, `pending_purge`, and the new columns on `folders` (`space_id`, `remote_id`, `mode`) and `notes` (`space_id`, `remote_id`, `author_ref`). Schema head asserts 25 -> 26. |
| C2 | T07 | The new `folders` and `notes` columns are declared in `sync/protocol/catalog.rs` so they travel between the devices of one account. |
| C3 | T07 | `spaces` and `pending_purge` are NOT in the sync catalog: a cursor and a purge intent are device-local. |
| C4 | T08 | `infrastructure/backend/spaces.rs` covers the 11 backend routes, follows the `shares.rs` client shape, and surfaces a uniform 404 as "no access", never as a hard error. |
| C5 | T09 | `application/space.rs` exposes join / pull / apply-delta / write. A write while offline is refused, never queued. |
| C6 | T09 | Effective mode is read from the pulled folder tree; the app never writes into a folder whose effective mode is `read` unless the caller owns the space. |
| C7 | T10 | Applying a delta creates real local notes: a received note goes through `note_persistence` and `embed::embed_note`, so it is searchable and usable in chat with no extra code. |
| C8 | T10 | A note carrying a `remote_id` is rewritten on each pull; a local edit by a non-author is never preserved against the server copy. |
| C9 | T11 | `pending_purge(note_id, kind, queued_at)` is written before the vector delete and cleared only on success. It is drained at boot and after each pull, until it succeeds. |
| C10 | T11 | `delete_note_embeddings` stops swallowing its error (`let _`, `embed/mod.rs:213`): a failure leaves a `pending_purge` row behind. |
| C11 | T11b | The P2P delete path (`sync/protocol/apply/entity.rs`) writes a `pending_purge` row instead of leaving the LanceDB vector alive. |
| C12 | T12 | Folder mode is visible in the UI, and the right to write is visible BEFORE writing (no failed write to discover a read-only folder). |
| C13 | T13 | A pull fires on app foreground and on first open of a space folder, with a 30 s floor per space. The last successful pull timestamp is shown. |
| C14 | T14 | Leaving or being revoked offers to keep one's notes: the kept copies get `space_id` NULL and `remote_id` cleared, becoming ordinary local notes. The removal option tombstones the leaver's own notes. |
| C15 | T14 | A note whose author left without removal stays, author greyed. |
| C16 | T15 | `wipe_local_content` covers `spaces` and `pending_purge`; backup and restore carry the space columns. |
| C17 | T16 | A two-device / two-account protocol is written into `verification-bundle-T07-T16.md`, up to zero ghost note after a removal. |

## Out of scope (never build in this run)

- Account onboarding and account creation from the app (brief task 1).
- Invitation UI, universal link, QR code (brief task 3).
- Fine-grained roles and invitation delegation.
- Editing a note one does not author.
- End-to-end encryption of a space.
- Merging an existing local folder into a space.
- Android, Windows, Linux.
- Audio transport for space voice notes (Q4: transcription only).
- Backend tasks T01-T06: shipped in `marketplace-flowflow`, branch `feat/spaces-backend`.

## Edit scope (authorized files)

```
src/infrastructure/persistence/schema.rs
src/infrastructure/persistence/note_repo.rs
src/infrastructure/sync/protocol/catalog.rs
src/infrastructure/sync/protocol/apply/entity.rs
src/infrastructure/backend/spaces.rs          (new)
src/infrastructure/backend/mod.rs
src/infrastructure/vectordb.rs
src/application/space.rs                      (new, may split into space/)
src/application/embed/mod.rs
src/application/note_persistence.rs
src/application/backup/
src/ui/sidebar/folders.rs
src/ui/notes/row_menu.rs
src/ui/notes/folder_picker.rs
src/ui/app/watchers.rs
src/lib.rs, src/prelude.rs                    (module wiring only)
tests/
docs/proposals/0002-collaborative-shared-folders/
```

## Execution order

```
T07 -> T08 -> T09 -> T10 -> T11 -> T11b -> T16
T09 -> T12, T13, T14
T10 -> T15
```

T09, T10, T13, T14 all write `application/space.rs`: serialized, never parallel.
