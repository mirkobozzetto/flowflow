---
proposal_id: "0008"
slug: "note-deletion-lifetime"
title: "Suppression fiable des notes : correction et preuves"
status: Accepted
format: standard
created: "2026-09-08"
scope_path: "src/ui/notes"
source_brief: "docs/brief/note-deletion-lifetime/"
---
# 0008 : Suppression fiable des notes : correction et preuves

## 1. Résumé

Fix confirmed note deletion by moving its task outside the closing menu's
lifetime, reporting persistence failures, and applying UI success only after
SQLite commits. Reuse Dioxus's existing `spawn_forever` pattern rather than
introducing a worker framework. Migrate affected callers to an explicit Result.

No SQLite schema/data migration is justified by the reproduced defect. This is
an application-lifetime and error-contract migration. The local implementation
is complete and verified: 38 tests pass, including a red/green differential
against the same final regression test. Details: [VALIDATION.md](VALIDATION.md).
Device installation, commits, publication and release were not performed.

## 3. Problème et motivation

### Executed baseline and isolated candidate

The extended integration test uses actual menu source, Dioxus events, SQLite,
LanceDB and a handshake-controlled loopback revoke endpoint. No personal data.
Full observations and limits: [VALIDATION.md](VALIDATION.md).

| Scenario | Original source | Isolated spawn_forever-only copy |
| --- | --- | --- |
| List deletion, all notes/theme | Pass | Pass |
| Open-note deletion, all/theme/subtheme/thread/detached | Fail | Pass |
| Mounted-menu control and cancellation | Pass | Pass |
| Slow revoke reaches server after menu closes | No | Yes |
| While revoke is pending, SQL note still exists | Task canceled | Yes, but already marked deleted |
| Completion preserves another note selection | Not reached | Fail: selection cleared |
| SQL abort preserves existing vectors | Fail: vectors deleted | Same failure |
| Retry after removing the SQL fault | Pass | Pass |

Both pre-implementation runs were red (1 passed, 1 failed): the minimal
candidate fixed lifetime cancellation but exposed the other required
protections. No app source was edited at that historical checkpoint. The full
correction and final green/differential results are recorded in VALIDATION.md.

SQLite-only membership checks pass in all five cases. The failure is not a
folder-ID issue in these scenarios. NoteMenu closes itself and starts a task
in its own scope (menu.rs:175-188); NoteDetail removes it (262-268).

Fault injection now proves the error-handling defect: a BEFORE DELETE trigger
rejects deletion; the note/audio survive but the purge is queued and LanceDB
loses the note vector. Removing the trigger and retrying deletes all three.
This is a synthetic reproduction, not evidence of an incident on the iPhone.

### Boundaries of the evidence

The installed iPhone build and exact gestures remain unverified. The executed
failure distinguishes open-note menu from list menu, not theme from All Notes.
If the same list-menu gesture fails only in a theme on the installed build,
this reproduction is not a complete explanation of that additional symptom.

The baseline is local dev at 09e34a5 plus existing unrelated dirty changes.
GitNexus reports a stale index and degraded impact traversal (staticGated).
Do not interpret its empty caller graph as safety evidence. Current source and
execution are authoritative. Do not fold unrelated sync/identity repairs into
this correction or claim these tests prove the installed release.

## 4. Objectifs et non-objectifs

- Delete the selected note from every local view while preserving other notes.
- Survive closing the menu and navigating away while revocation is pending.
- Keep cancellation harmless and prevent duplicate pending confirmations.
- Preserve SQL rollback, files and vectors on failed SQL deletion.
- Show an actionable, localized error instead of a false successful exit.
- Retain existing share-revocation policy and sync scheduling.
- Prove the expected behavior red before correction and green afterward.

Excluded: schema changes, ID reassignment, mass data repair, new dependencies,
a generic job queue, unrelated sync redesign, and remote publication policy
changes. Process termination while a network request is pending is not solved
by `spawn_forever`; do not promise crash-durable jobs.

## 5. Alternatives envisagées

| Option | Assessment |
| --- | --- |
| Repair folder/thread references | Rejected: tested references and deletion work; no corruption evidence. |
| Keep NoteMenu permanently mounted | Useful causal control, but unnecessarily changes component ownership and does not protect against leaving NoteDetail. |
| Replace spawn alone | Fixes the proved cancellation, but leaves premature success, swallowed SQL errors and potentially stale component signals. Incomplete for the brief. |
| Root-lifetime task plus explicit deletion result | Recommended: existing native pattern, bounded change, testable failure contract. |

## 6. Conception retenue

### Persistence contract and side-effect ordering

Change `note_persistence::delete_note` to `Result<(), String>`:

1. Run existing `delete_note_rows` through `with_tx` and propagate failure.
2. Only after commit, call existing `finish_note_delete` and return success.
3. Keep the durable vector-purge mechanism; vector cleanup can remain pending.
   SQL success does not mean every asynchronous vector operation has finished.
4. Keep deletion idempotent when the target was already deleted.

Keep `delete_note_rows`'s current signature and transaction composition. No
new fallback, compatibility wrapper or second deletion implementation.

### UI lifetime, navigation and error state

- In NoteMenu use `dioxus::core::spawn_forever`, already used for unmount-safe
  work in chat/menu.rs, sidebar/mod.rs and notes/related.rs.
- Capture owned note ID, Arc<Database> and Arc<SyncEngine> before leaving the
  handler. Do not dereference menu-owned signals after an await.
- Close the menu, but do not set `deleted=true` or start success navigation
  before SQLite succeeds. Keep the current detail visible while pending.
- Store the pending deletion ID and user-facing error at AppState lifetime,
  not in the disappearing menu. One pending operation is sufficient; disable
  duplicate confirmation until completion. No general task registry.
- Preserve current NoteMenu revocation ordering and best-effort policy: await
  revocation, then perform local deletion. Capture any revocation failure and
  show a distinct warning; never claim the public link was revoked on failure.
  Do not silently change list-menu sharing semantics in this fix.
- After SQL success, invalidate DB-backed note views and schedule sync. Mark
  the still-mounted matching detail deleted before its save-on-drop runs.
  Guard writes to detail-owned signals if it has already unmounted.
- Only navigate away if the user still views the deleted note. Guard the
  existing delayed exit too: completion must not close a different note or
  leave sliding_out stuck after an intervening navigation.
- On SQL failure clear pending state, leave `deleted=false`, preserve the
  user's editor content, and show a dismissible root-lifetime error banner
  using the existing design-system styles and English/French translations.
- NoteRowMenu keeps its root mounting pattern; migrate its error handling so
  failed deletion does not masquerade as a successfully updated list.

### Complete caller migration

| Caller | Required treatment |
| --- | --- |
| ui/notes/menu.rs | Branch on Result; success-only deleted flag/navigation/sync. |
| ui/notes/row_menu.rs | Branch on Result; retain note and show error on failure. |
| application/sharing.rs::align_kept_content | Emit RemovedByAuthor only after success. Preserve provenance on failure so the next alignment can retry. Remove redundant provenance deletion already handled transactionally. |
| application/space/mod.rs::detach_locally | Return Result and stop before removing remaining space/folder metadata when a local operation fails. Propagate touched detach/delete errors, not just the note call. |
| space::leave / stop_sharing | Map local cleanup failure to SpaceError instead of returning success. Existing remote-first ordering remains. |
| application/space/pull.rs revoked branch | Propagate detach failure; do not report successful local cleanup. Preserve the existing dirty pull implementation. |
| tests/pending_purge_test.rs, space_publish_test.rs, space_leave_test.rs | Assert the returned Result while retaining behavioral checks. |
| application/space/pull.rs delete_note_rows path | Signature unchanged; retain its transaction and deferred side effects. |

Re-run references/text callsite inventory before implementation because the
checkout is dirty and may change. Direct Database::delete_note already returns
Result and is not an API migration target.

### Data and delivery migration

No migration number, SQL repair or backfill. Existing notes retain their IDs,
folder associations and thread IDs. A deletion canceled by this defect leaves
the note present: after installing the correction, explicitly delete it again.
Never infer old deletion intent and automatically remove existing notes.

No server/schema rollout is required for this local fix. Rollback is a code
revert to the prior build, not a database downgrade. A successful user deletion
is not undone by reverting code. Obtain installation/publication approval
separately and retain the existing user data.

## 7. Inconvénients et risques

- The integration host now uses the actual exit/autosave hooks and tests
  delayed navigation and error/retry. It is not the entire native screen.
- A detached task is safe only if its data and UI state survive its owner;
  blindly replacing spawn while capturing disposable signals is insufficient.
- Revocation and SQLite cannot commit atomically. A link may be revoked before
  local deletion fails; retry must accept already-revoked links. Conversely,
  best-effort network failure can leave a live link after local deletion.
  Surface this existing limitation; durable remote revocation is separate scope.
- Space cleanup remains a sequence, not a newly atomic operation. On failure,
  retain metadata needed for retry and prove retry converges after partial work.
- Test storage must remain isolated even for global Database/VectorStore opens.
  Use a loopback synthetic server only for network-boundary tests, with no real
  accounts or peers. Never depend on an unrelated occupied beacon port.
- Exact user-device reproduction is a release gate. If same-gesture themed
  deletion still fails, stop and investigate that residual case, not SQLite
  migrations based on the original suspicion.

## 9. Recommandation et justification

The experiment validated root-lifetime execution AND guarded completion/error
handling, rejecting the spawn-only shortcut. Mirko requested continuation
through the correction and tests. That local work is complete: the diagnostic
target and all five targeted suites pass. The same final regression fails with
only lifetime protection removed and passes after restoration. The proposal is
Accepted; the native installation/release gate remains unperformed.

## 10. Plan d'implémentation

| ID | Title | Files | Depends on | Effort |
| --- | --- | --- | --- | --- |
| T1 | Freeze source boundary and extend red integration evidence | note_delete_integration_test.rs; existing targeted tests | Approval | S |
| T2 | Migrate deletion Result and all affected callers | note_persistence.rs; sharing.rs; space/mod.rs; space/pull.rs; affected tests | T1 | S |
| T3 | Correct UI lifetime, pending/error state and success ordering | notes/menu.rs; row_menu.rs; detail/mod.rs; state.rs; app/router.rs; en/fr.ftl | T2 | S |
| T4 | Prove red/green and failure recovery end to end | Integration tests; targeted existing suites | T3 | S |
| T5 | Verify native candidate and close evidence trace | Candidate build; brief evidence | T4; installation approval | S |

### T1: baseline and red cases

Preserve the dirty worktree and agree on a clean candidate boundary before
editing overlapping paths. Reuse executed evidence; rerun only after changes.

- DONE: real-menu theme/subtheme/thread/detachment and cancellation cases.
- DONE: SQLite abort, actual LanceDB deletion, audio preservation and retry.
- DONE: delayed loopback revoke, navigation, premature success and lost selection.
- DONE: duplicate confirmation, real exit/autosave hooks, and navigation during
  exit. Whole-screen native rendering remains T5, not a VirtualDom claim.
- DONE: SQL errors rendered through both menus, explicit Result handling,
  delayed revoke success/failure, and partial space-cleanup retry.
- DONE: original deletion assertions retained; final differential goes red
  without lifetime protection and green again after restoration.
- All production callers were migrated. The shared-content alignment branch
  was compiled, but not separately exercised against a faulting remote share.

### T2-T3: migrate and correct

Run LSP references and GitNexus impact before edits. If graph tooling remains
degraded, report UNKNOWN and confirm every caller through current source.
Apply the design above without touching unrelated dirty work. Do not weaken
assertions or keep the test's mounted-control behavior in production.

### T4: required proof

```sh
PATH="$HOME/.cargo/bin:$PATH" cargo test -j 8 \
  --test note_delete_integration_test -- --nocapture --test-threads=1
PATH="$HOME/.cargo/bin:$PATH" cargo test -j 8 \
  --test thread_test --test pending_purge_test --test space_publish_test \
  --test space_leave_test --test sync_protocol_test
```

Use the project's formatting/type checks for touched code. With only the fix
removed in an isolated copy, the same regression must fail; restored fix must
pass. Never stash unrelated dirty files. Fault cases must prove rollback AND
successful retry, not just an error string. Verify peer deletion convergence
with disposable databases and existing sync helpers if tombstone paths change.

### T5: native and delivery gate (not performed)

T1-T4 are complete locally; see VALIDATION.md for exact executed coverage.
T5 requires separate installation approval and is not part of the completed
local-test claim.

Record candidate revision and installed version/build. With disposable notes,
exercise the same delete gesture in All Notes, theme, subtheme, thread and
after detachment. Also exercise immediate navigation and an edited unsaved
note. Reopen the views to verify absence and verify unrelated notes remain.
Present baseline/fixed outputs and native observations. No release until these
pass; no automatic push, deployment or migration of personal data.
