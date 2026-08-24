---
artifact: "docs/proposals/0002-collaborative-shared-folders/PROPOSAL.md"
artifact_kind: "propose"
locked: "2026-08-24"
run: "T01-T06 (backend, marketplace-flowflow)"
repo: "/Users/mirkobozzetto/code/marketplace-flowflow"
---

# Definition of Done: spaces backend (proposal 0002, T01-T06)

> Immutable target. Every item below is a concrete, checkable condition the
> final verification bundle validates against. Requirement changes get a NEW
> entry; never silently rewrite an existing line.

## Decisions locked before the run (answers to PROPOSAL section 8)

| Q | Answer | Applies to |
|---|--------|-----------|
| Q2 | A space has no expiry: it lives while the owner is premium. Owner loses premium -> space becomes read-only, nothing is deleted. An INVITE code does expire. | T01, T03, T06 |
| Q3 | Caps: 20 members/space, 5000 notes/space, 64 KB/note. `pull` paginates at 200 rows and is floor-limited to 1 call / 30 s / device / space. | T01, T05, T06 |
| schema | `space_notes.content` is the LAST column, so reads of `seq`/`deleted_at` never walk its overflow chain. | T01 |

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | Migration 18 creates `spaces`, `space_members`, `space_folders`, `space_notes`, `space_invites`; index on `(space_id, seq)`; `spaces.seq_counter` present; `space_notes.content` last column | T01 | `cargo test` + fresh-db boot |
| C2 | `effective_mode(f)` = `collab` only if `f` and every ancestor are `collab`, else `read`; unit-tested standalone | T02, §6.2 | `cargo test spaces::perm` |
| C3 | Write guard: owner writes anywhere; member writes where effective mode is `collab`; a member only edits/deletes notes they authored | T02, §6.2 | `cargo test spaces::perm` |
| C4 | Depth cap of 8 ancestors enforced; a move under one's own descendant is refused | T02, §6.2 | `cargo test spaces::perm` |
| C5 | `POST /v1/spaces` requires a linked web_user AND owner premium; `join`/`pull` do NOT test the caller's premium | T03, §6.3 | `cargo test` route tests |
| C6 | Every spaces route answers a uniform 404 when the caller has no right on `space_id` (unknown / never joined / revoked) - never 403 | T03, §6.3 | `cargo test` route tests |
| C7 | An invite code is single-use: `consumed_at` set on first successful join makes it inert; it also expires | T03, §6.1 | `cargo test` route tests |
| C8 | `member/remove` and `leave` exist; a re-joining member reuses their `space_members` row (`removed_at` back to NULL) and restarts at cursor 0 | T03, §6.1 | `cargo test` route tests |
| C9 | folder / folder.move / folder.delete / note / note.delete routes exist and enforce C2-C4 | T04, §6.3 | `cargo test` route tests |
| C10 | `seq` is drawn from `spaces.seq_counter` via `UPDATE ... RETURNING` inside the SAME transaction as the write; two concurrent writes never share a number | T04, §6.1 | `cargo test` concurrency test |
| C11 | `folder.delete` tombstones the whole subtree; `note.delete` NULLs `title` and `content` but keeps the row | T04, §6.1, §6.2 | `cargo test` route tests |
| C12 | `POST /v1/spaces/pull` returns folders and notes changed since `since_seq`, tombstones included, plus `next_seq`; paginated at 200 rows with resume by `seq` | T05, §6.3 | `cargo test` route tests |
| C13 | Caps enforced server-side: 20 members, 5000 notes, 64 KB note body; over-cap writes are refused, not truncated | T06 | `cargo test` route tests |
| C14 | `pull` is rate-limited per device+space (30 s floor) on top of the global `ratelimit::layer` | T06, §6.3, §6.5 | `cargo test` route tests |
| C15 | Owner without premium: space goes read-only (writes refused, `pull` still serves) | T06, Q2 | `cargo test` route tests |

## Out of scope (never build)

- Account onboarding from the app (brief task 1)
- Invite UI, universal link, QR code (brief task 3)
- Fine-grained roles, invite delegation
- Editing a note you did not author
- End-to-end encryption of a space
- Merging an existing local folder into a space
- Everything app-side: T07-T16 (migration V26, backend client, `application/space.rs`,
  `pending_purge`, UI, backup) - a separate run in the `flowflow` repo
- Android, Windows, Linux

## Edit scope

Repo `marketplace-flowflow` only:

- `src/db/migrations.rs`
- `src/features/spaces/` (new module: `mod.rs`, `perm.rs`, `routes.rs`, `repo.rs`)
- `src/lib.rs` (route mounting)
- `src/gate.rs`
- `src/ratelimit.rs`
- `tests/` (route + perm tests)

No file in the `flowflow` repo is touched by this run, except this proposal
folder's own ledger files.
