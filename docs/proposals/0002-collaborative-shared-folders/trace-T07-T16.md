---
artifact: "docs/proposals/0002-collaborative-shared-folders/PROPOSAL.md"
artifact_kind: "propose"
engine_tier: "solo"
repo: "/Users/mirkobozzetto/code/flowflow"
work_branch: "feat/spaces-app"
base_branch: "dev"
commit_mode: true
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: "shipped"
updated: "2026-08-24"
---

# Trace Ledger: spaces app side (proposal 0002, T07-T16)

> Single source of truth for progress. A fresh session reads ONLY this file to
> resume. Run scoped to T07-T16; T01-T06 are a separate run in the
> `marketplace-flowflow` repo (`trace-T01-T06.md`, shipped).

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Commit | Notes |
|------|---------------|--------|---------------|--------|--------|-------|
| T07 | C1, C2, C3 | done | `persistence/schema.rs`, `sync/protocol/{catalog,mod}.rs` | solo | `c8616b1` | V26; space columns declared in the catalog, `spaces`/`pending_purge` out of it; PROTOCOL_VERSION 4 -> 5 (a v4 peer has no schema for the new columns) |
| T08 | C4 | done | `infrastructure/backend/{spaces,mod}.rs` | solo | `98703f9` | 11 routes, `shares.rs` shape; `is_read_only()` / `is_limit()` tell a frozen space from a dead one |
| T09 | C5, C6 | done | `domain/space.rs`, `persistence/space_repo.rs`, `application/space/{mod,pull,write}.rs` | solo | `b6a3112` | write goes to the server then pulls the row back (one write path); offline write refused, never queued |
| T10 | C7, C8 | done | `application/space/pull.rs`, `domain/note.rs`, `persistence/note_repo.rs` | solo | `b6a3112` | a pulled note is an ordinary note through `note_persistence` + `embed_note`; folders applied in two passes so a child never hangs at the root |
| T11 | C9, C10 | done | `persistence/chunk_repo.rs`, `application/embed/mod.rs`, `note_persistence.rs`, `ui/app/boot.rs`, `sync/reconcile.rs` | solo | `c7ebe3e` | `pending_purge` queued BEFORE the vector delete, cleared only on confirmation; drained at boot, after each pull, and in the reconcile pass |
| T11b | C11 | done | `sync/protocol/apply/entity.rs` | solo | `c7ebe3e` | the P2P applier queues the purge instead of leaving the vector alive |
| T12 | C12 | done | `domain/folder.rs`, `persistence/folder_repo.rs`, `application/space/write.rs`, `ui/sidebar/folders.rs`, `ui/notes/note_list.rs`, `ui/app/fab.rs`, `i18n/locales/*.ftl` | solo | `f0ef03e` | one `folder_right()` feeds both the badge and the compose button, so they can never disagree |
| T13 | C13 | done | `domain/space.rs`, `application/space/pull.rs`, `ui/app/watchers.rs`, `ui/mod.rs`, `platform/ios/sync_ffi.rs` | solo | `ad2a1aa` | 30 s floor as a pure function; foreground observer + 30 s watcher loop |
| T14 | C14, C15 | done | `application/space/mod.rs`, `tests/space_leave_test.rs` | solo | `b6a3112`, `50fa0e2` | keep = detach mine (local id unchanged, embeddings stay valid), drop theirs; withdraw = tombstone mine server-side |
| T15 | C16 | done | `persistence/note_repo.rs`, `tests/account_wipe_test.rs` | solo | `50fa0e2` | wipe covers `spaces` + `pending_purge`; backup needs no code (whole-file snapshot, no table allowlist in `validate.rs`) |
| T16 | C17 | done | `docs/proposals/0002-.../verification-bundle-T07-T16.md` | solo | pending | 10-section two-device protocol, gated on the backend deploy |

Suite: 719 passed, 12 ignored (was 709 before T11). `cargo clippy
--features mobile --all-targets` clean apart from one pre-existing warning
(`rag/mod.rs`, `period_empty`). `cargo build --features mobile` green.

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-01 | open questions | resolved | Q4 answered by Mirko: a space voice note travels as transcription only, no audio crosses the backend. Q6 answered: a note whose author left without withdrawing stays, author greyed. Q1 and Q5 block no task in this run. |
| step-02 | branch | `feat/spaces-app` from `dev` | `dev` carries one unpushed commit (`11659fb`, docs 0002); it must be pushed before the PR or it shows up in the diff. |
| step-04 | schema | V26 edited twice before any device ran it | `spaces.mode` was dropped (no use: read-only is learned from a 409), then `owner_ref` became `is_owner` (the pull payload never names the owner, so ownership is recorded at creation). Safe: the branch was never installed. |
| step-04 | protocol | PROTOCOL_VERSION 4 -> 5 | The space columns ride in the note and folder catalog rows. A v4 peer has no schema for them and its apply would fail on an unknown column, the same reasoning the v4 bump already recorded. |
| step-04 | signature | `delete_note_embeddings` / `delete_attachment_embeddings` now take `&Database` | They used to reopen the app-wide database from inside a detached thread, which made the queue untestable and the caller's handle unused. Both call sites already had a handle. |
| step-05 | test flake | reported, not fixed | `sync_data_version_test::data_version_bumps_on_outbound_apply` failed once under full-suite load, passes alone and on rerun. Timing, unrelated to this run. |

## HALT events

- none

## Blocking on

The device protocol cannot start until `marketplace-flowflow`
`feat/spaces-backend` is merged and deployed: migration 18 and the
`/v1/spaces/*` routes do not exist in production yet.
