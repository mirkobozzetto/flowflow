# Ship trace: thread note context menus

Proposal 0009 accepted by Mirko on 2026-09-08 with targeted tests and progressive
commit/push delivery. Solo execution. No public release or unrelated sync work.

## Source boundary

- Started on dev at 09e34a5 with preexisting deletion and sync/identity changes.
- Fetched origin/dev and origin/main. Fast-forwarded dev to 527bd1a: identical
  source tree, ancestry only. No branch switch or user-work stash.
- Publish the earlier, verified deletion correction as a prerequisite, without
  the unrelated sync/identity changes. Only its Result propagation hunk from
  space/pull.rs is staged; all identity changes remain local.
- Verify selected commit contents in a disposable detached worktree. The only
  ignored build input copied there is the required generated tailwind.css.

## Units

| Unit | State | Evidence |
| --- | --- | --- |
| P0: deletion prerequisite | Published: 487b803 | Isolated source: 37 tests pass across deletion, purge, space leave/publish, thread and sync suites. The extra local sync-repair test is deliberately excluded. |
| T1: stale menu-page regression | Reproduced | Real NoteRowMenu: abandoned Move/Delete confirmation, same/different note, all four reopenings fail before T2. Target: 1 passed, 1 failed. Retain this test with T2 to keep published commits green. |
| T2: session-owned note-menu page | Published: 54bea8f | The same four cases pass in the isolated staged source. Deletion target: 2 passed, 0 failed, including all earlier rollback/retry/navigation scenarios. Permanent root owner retained. GitNexus staged analysis: high, 7 flows; compiler and real integration exercised affected menu paths. |
| T3: thread Copy/Share gestures | Published: 90a22ba | Isolated staged source: 3 real ThreadNode event tests and 2 deletion/menu integration tests pass. Covers right-click, mouse/touch hold, release-click suppression, subsequent normal click, scroll/cancel/leave/unmount, two targeted note menus, Copy/Share only, return thread and unchanged content/membership. Clipboard OS write and real share-link presentation require native checks; no personal-note publication performed. |
| T4: targeted/non-regression checks | Passed: 45 tests | Clean published source 90a22ba: 3 gesture tests, 2 deletion/menu tests, 5 folder-tree, 4 pending-purge, 5 space-leave, 3 space-publish, 12 sync-protocol and 11 thread tests. No failures. No personal data used or published. Native checks accompany T5. |
| T5: installed apps/manual handoff | Pending | make all and make desktop-app, no data reset. |

GitNexus detect-changes executed for all and staged scopes. Staged prerequisite
reports critical (21 symbols, 67 processes), including deletion/space callers.
Impact traversal remains degraded/UNKNOWN; no graph all-clear. Exact staged
source and the targeted integration executions provide the behavioral evidence.
