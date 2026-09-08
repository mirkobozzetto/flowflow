# Validation of proposal 0008

Date: 2026-09-08. Local implementation is now verified: 38 tests pass across
six targeted suites. The final regression target also fails when only the
lifetime correction is removed, then passes again after restoration.

Tests use disposable SQLite, audio files, LanceDB, synthetic credentials and
a loopback HTTP server. No personal database, installed app or remote service
was modified. The sections below retain the original pre-implementation
experiments; final implementation results follow them.

## Experiment

1. Extend `tests/note_delete_integration_test.rs` against the actual source.
2. Run the extended baseline.
3. Copy `src/ui/notes/menu.rs` to a temporary directory and change exactly one
   call: `spawn(async move {` to `dioxus::core::spawn_forever(async move {`.
4. Temporarily point the test's NoteMenu module at that copy; run the same
   assertions. No other candidate change, especially none to persistence.
5. Restore the original module path, format only the test, rerun the baseline,
   and delete the temporary source copy.

Command for all three executions:

```sh
PATH="$HOME/.cargo/bin:$PATH" cargo test -j 8 \
  --test note_delete_integration_test -- --nocapture --test-threads=1
```

`spawn_forever` is not an infinite loop. Its future survives removal of the
component that spawned it, but finishes when the async body finishes. It does
not survive process termination and does not make scoped signals immortal.

## Original source: observed stdout

```text
UI list/all: exists=false, expected=false
UI list/theme: exists=false, expected=false
UI detail/all: exists=true, expected=false
UI detail/theme: exists=true, expected=false
UI detail/subtheme/thread: exists=true, expected=false
UI detail/subtheme/detached: exists=true, expected=false
UI detail/mounted-control: exists=false, expected=false
UI detail/cancel: exists=true, expected=true
UI list/cancel: exists=true, expected=true
DELAYED revoke: request never reached loopback server
SQL ABORT: note_preserved=true, audio_preserved=true, purge_queued=true, vectors_preserved=false
SQL RETRY: note_deleted=true, audio_deleted=true, vector_deleted=true
test result: FAILED. 1 passed; 1 failed
```

The final rerun after restoring the original import has the same failures.
The separate SQLite test passes for all five original folder/thread cases.

## Isolated spawn_forever-only candidate: observed stdout

```text
UI list/all: exists=false, expected=false
UI list/theme: exists=false, expected=false
UI detail/all: exists=false, expected=false
UI detail/theme: exists=false, expected=false
UI detail/subtheme/thread: exists=false, expected=false
UI detail/subtheme/detached: exists=false, expected=false
UI detail/mounted-control: exists=false, expected=false
UI detail/cancel: exists=true, expected=true
UI list/cancel: exists=true, expected=true
DELAYED pending: SQL note exists=true, marked_deleted=true
DELAYED after navigation: deleted=true, other_selection_preserved=false
SQL ABORT: note_preserved=true, audio_preserved=true, purge_queued=true, vectors_preserved=false
SQL RETRY: note_deleted=true, audio_deleted=true, vector_deleted=true
Deletion contract violations: ["marked deleted before SQL commit", "completion cleared another note selection", "SQL rollback still purged live note vectors"]
test result: FAILED. 1 passed; 1 failed
```

The delayed server signals that it received the real revoke HTTP request and
withholds its response. The test navigates to another note, unmounts the old
detail host, then releases the response. The candidate deletes the intended
note but clears `current_note_id` for the newly selected note. The other note's
SQL row remains intact: this is selection loss, not deletion of the other note.

## SQL rollback and retry proof

A temporary BEFORE DELETE trigger raises `ABORT` inside the real SQLite path.
`delete_note_rows` returns the injected error. Calling the current public
`delete_note` then leaves the row and audio intact but queues a purge and
actually removes its seeded vector from LanceDB.

The test restores the synthetic vector, removes the trigger and retries.
SQLite note/audio rows, audio file and vector are then deleted. Waiting for
that vector deletion prevents disposal of the temporary store while cleanup
is still running. No conclusion about an incident in personal data is implied.

## Decision supported by the experiment

- Accept root-lifetime execution as a suitable mechanism for menu cancellation.
- Reject a standalone spawn-to-spawn_forever substitution as a complete fix.
- Require success only after SQL commit, guarded navigation/selection writes,
  explicit persistence errors and no vector cleanup after SQL rollback.
- Do not migrate folder/thread identifiers based on this defect.

At this pre-implementation checkpoint, the proposal moved to Review and the
retained test remained red against original source. The later implementation
and its green proof are recorded below; this historical experiment alone
was not a release checkpoint.

## Implementation results

The source now propagates SQL errors before starting destructive side effects.
All affected deletion/space-cleanup callers handle the Result. NoteMenu uses
root-lifetime execution, root pending/error state, success-only deletion flags
and guarded selection/navigation. The real exit and save-on-drop hooks are
used by the integration host. English/French errors use existing UI styles.

Observed outcomes:

- All nine original UI cases pass: list/open-note entry points, theme,
  subtheme/thread, detached note, mounted control and both cancellation paths.
- Slow revoke, with both HTTP 204 and HTTP 503: no premature deleted flag,
  exactly one request despite double confirmation, intended note deleted,
  another note's selection preserved, and warning only on revoke failure.
- SQL failure through each actual menu renders the root error message, clears
  pending state, preserves note/editor content and avoids false success.
  Removing the fault and retrying deletes the note; the real exit hook finishes.
- Navigating to another note during the 150 ms exit preserves the newer view
  and clears sliding_out.
- SQL abort preserves the note, audio file and seeded LanceDB vector, without
  queuing a purge. Retry removes SQL rows, audio file and actual vector.
- Partial space withdrawal preserves the failing note and space metadata;
  retry after removing the SQL fault completes cleanup.
- The separate five-case SQLite relationship test preserves other thread members.

| Target | Passed | Failed |
| --- | ---: | ---: |
| note_delete_integration_test | 2 | 0 |
| pending_purge_test | 4 | 0 |
| space_leave_test | 5 | 0 |
| space_publish_test | 3 | 0 |
| sync_protocol_test | 13 | 0 |
| thread_test | 11 | 0 |
| Total | 38 | 0 |

The six-target run redirected FLOWFLOW_DATA_DIR and FLOWFLOW_VECTORDB_PATH
to a new temporary directory. The diagnostic target additionally owns its
SQLite/audio/vector directories internally. Touched Rust files were formatted;
these test builds compiled the changed library and migrated callers.

## Final differential proof

Copy the fixed NoteMenu into a temporary directory and replace exactly its
spawn_forever call with scoped spawn. Point the unchanged regression test at
that copy, keeping all Result APIs and assertions intact. This avoids stashing
any of the user's unrelated dirty work.

- Fixed source: 2 passed, 0 failed.
- Lifetime correction removed: 1 passed, 1 failed. Four open-note deletion
  cases again leave the row present; slow revoke never reaches the server.
  The UI error assertion also fails because its task never executes.
- Original production import restored: 2 passed, 0 failed.

The temporary source copy and test-owned stores were removed after execution.

## Delivery boundary

The real menu components, error component, exit/autosave hooks, SQLite and
LanceDB were exercised through Dioxus VirtualDom integration, not a native
WebView or the entire NoteDetail screen. No iPhone build was installed or
visually verified. Native gestures and installed-version verification remain
an explicitly separate, unperformed installation/release gate. There was no
commit, push, deployment, schema migration or personal-data repair.

The test deliberately reserves its own UDP beacon port, so the engine's
Address already in use message is test-owned isolation, not a dependency on
another running app. TCP uses an ephemeral port and an unpaired synthetic DB.
