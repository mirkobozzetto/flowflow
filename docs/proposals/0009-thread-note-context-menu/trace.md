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
| P0: deletion prerequisite | Verified for commit/push | Isolated source: 37 tests pass across deletion, purge, space leave/publish, thread and sync suites. The extra local sync-repair test is deliberately excluded. |
| T1: stale menu-page regression | Pending | No tests added yet. |
| T2: session-owned note-menu page | Pending | Preserve permanent root execution owner. |
| T3: thread Copy/Share gestures | Pending | No move/delete entry in this menu. |
| T4: targeted/native checks | Pending | No personal-note publication. |
| T5: installed apps/manual handoff | Pending | make all and make desktop-app, no data reset. |

GitNexus detect-changes executed for all and staged scopes. Staged prerequisite
reports critical (21 symbols, 67 processes), including deletion/space callers.
Impact traversal remains degraded/UNKNOWN; no graph all-clear. Exact staged
source and the targeted integration executions provide the behavioral evidence.
