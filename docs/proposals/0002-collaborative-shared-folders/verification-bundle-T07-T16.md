---
artifact: "docs/proposals/0002-collaborative-shared-folders/PROPOSAL.md"
run_id: "-T07-T16"
repo: "/Users/mirkobozzetto/code/flowflow"
work_branch: "feat/spaces-app"
stack: "rust / cargo"
written: "2026-08-24"
---

# Verification bundle: spaces app side (proposal 0002, T07-T16)

Everything below is run by Mirko. Nothing here touches a database or deploys.

## 0. Prerequisite, and it is a hard one

The backend half (T01-T06) is written but NOT deployed. It lives in
`marketplace-flowflow`, branch `feat/spaces-backend`, and it carries
migration 18 plus the 11 `/v1/spaces/*` routes.

Until it reaches production, every space call from the app answers 404 and
the device protocol below cannot start.

```bash
cd /Users/mirkobozzetto/code/marketplace-flowflow
git log --oneline dev..feat/spaces-backend
# then: PR -> main -> Dokploy deploy
```

Two accounts are also needed, each with a linked web identity, and the OWNER
account must be premium. Joining does not require premium; creating does.

## 1. Automated checks (this repo)

```bash
cd /Users/mirkobozzetto/code/flowflow
cargo fmt --check
cargo clippy --features mobile --all-targets
cargo test --features mobile
```

Expected: 719 passed, 12 ignored. Clippy reports one pre-existing warning in
`src/application/rag/mod.rs` (`period_empty`), untouched by this run.

The space-specific suites, if you want them alone:

```bash
cargo test --features mobile --test space_schema_test    # V26 + sync catalog
cargo test --features mobile --test space_client_test    # backend wire shapes
cargo test --features mobile --test space_delta_test     # perms + delta + cadence
cargo test --features mobile --test space_leave_test     # leaving, keeping, withdrawing
cargo test --features mobile --test pending_purge_test   # replayable vector purge
cargo test --features mobile --test account_wipe_test    # wipe covers space rows
```

Known flake, not caused by this run: `sync_data_version_test::
data_version_bumps_on_outbound_apply` can fail once under full-suite parallel
load (it reads `activity()` while the session is still winding down). It
passes alone and on a second full run.

## 2. Install on device

```bash
cd /Users/mirkobozzetto/code/flowflow
make all
```

Two devices are needed for section 4. A Mac build (`make desktop-app`) can
stand in for the second one.

## 3. Migration V26, first launch

1. Launch the app on a device that already holds notes.
2. `make logs`, filter FlowFlow: `[db] applying migration v26` appears once.
3. Reopen the app: it does NOT appear again.
4. Existing notes, themes and reminders are all still there.

Fails if: the migration replays, or any pre-existing content vanished.

## 4. The two-device, two-account protocol

Device A = account OWNER (premium). Device B = a second account, not premium.
There is no invitation UI in this run (brief task 3, out of scope), so the
code is minted and consumed through the code paths, not through a screen.

### 4.1 Create and join

1. A creates a space, then a theme inside it declared collaborative.
2. A mints an invite code and passes it to B.
3. B joins with that code.

Expected on B: the space theme appears with a `Partagé` badge, and A's notes
land in it within 30 seconds.

Fails if: B sees an empty theme (a join must pull immediately, not wait for
the next cadence tick).

### 4.2 A note added by one member reaches the other

1. B writes a note in the collaborative theme.
2. Wait, or reopen the app on A.

Expected on A: the note appears, with no republishing and no code change.
That is the whole point of the feature.

### 4.3 The note is an ordinary note

On A, with the received note in view:

1. Search for a word from its body in the notes search: it is found.
2. Open the chat and ask a question its content answers: it is cited as a
   source.

Fails if: the note is visible but never searchable. It would mean the embed
pipeline was skipped, and every claim about chat in the proposal falls.

### 4.4 A read-only theme really refuses

1. A creates a second theme declared read-only, and a subtheme UNDER it
   declared collaborative.
2. On B, open both.

Expected on B: both carry the `Lecture seule` badge (the subtheme declares
collab and its parent overrides it), the notes list shows the read-only
line, and no compose button is offered on either.

Fails if: the subtheme shows as writable. The restriction must descend
without any child row being rewritten.

### 4.5 Deletion travels to zero ghost note

This is the section that matters legally.

1. B deletes one of its own notes in the space.
2. On A, wait for the pull (30 s at most, or reopen the app).
3. On A: the note is gone from the list.
4. On A: search for a word that ONLY that note contained. Nothing comes back.
5. On A: ask the chat a question that only that note answered. It must not be
   cited, and must not be quoted.

Fails if: the note is gone from the list but still answers in chat. That is
the ghost note the whole design exists to prevent, and it is a compliance
defect, not a bug to file for later.

### 4.6 The purge survives a failure

1. On A, put the device in airplane mode, then delete a space note locally.
2. Force-quit the app.
3. Relaunch with the network back.

Expected: `make logs` shows `purge drain: N cleared`, and the note's content
is not searchable and not quotable in chat.

Fails if: nothing drains at boot. The queue exists precisely so a failed
LanceDB call is retried instead of forgotten.

### 4.7 Leaving keeps what was promised

1. On B, leave the space choosing to KEEP its own notes.

Expected on B: its own notes are still there, now as ordinary notes with no
badge and no space; the notes written by A are gone, along with their
vectors (search and chat both come up empty on them).

2. Repeat with a second member choosing to WITHDRAW instead.

Expected on A: the withdrawing member's notes disappear from A's device too,
list, search and chat alike.

### 4.8 A revoked member stops receiving

1. On A (owner), revoke B.
2. On B, reopen the app.

Expected on B: the pull answers the uniform 404 and the space stops
refreshing. B is told nothing about whether the space still exists.

### 4.9 P2P echo does not leave a vector behind

Two devices of the SAME account, both holding a space note.

1. Delete the note on device 1.
2. Let the P2P sync reach device 2.
3. On device 2: the note is gone from the list, and its content is not
   quotable in chat.

Fails if: chat still quotes it. Before this run the P2P applier deleted the
SQLite chunks only and never touched LanceDB.

### 4.10 Delete my data

1. On a device holding a space, Settings -> delete my data.
2. Reopen the app.

Expected: no space remains, no pull runs, the app boots normally with its
settings and device identity intact.

## 5. What this run does NOT cover

- Account onboarding from the app (brief task 1).
- Invitation UI, universal link, QR code (brief task 3).
- Audio for space voice notes: transcription only, decided at the run gate.
- Backend tasks T01-T06: shipped in `marketplace-flowflow`, verified there.
- Backup and restore of the space columns: the archive is a whole-file SQLite
  snapshot and `validate.rs` carries no table allowlist, so the columns travel
  by construction. Confirm anyway during 4.x if you run a restore.
