---
type: tasks
source_brief: brief.md
slug: note-deletion-lifetime
created: 2026-09-08
---

# Tasks

Following the initial diagnosis, Mirko requested continuation through a tested
correction. Local implementation and integration verification are complete.
Installation and publication remain separate and were not performed.

- [x] Compare list-menu and open-note-menu deletion paths.
- [x] Exercise SQLite theme/subtheme/thread relationships and detachment.
- [x] Reproduce failed deletion with real menu components and Dioxus events.
- [x] Establish the lifetime cause with an otherwise identical mounted control.
- [x] Test an isolated spawn_forever-only copy: cancellation fixed, but premature
  success and clearing another note selection reproduced during delayed revoke.
- [x] Inject SQL abort and prove actual vector loss despite rollback; verify retry.
- [x] Make confirmed deletion independent of the closing menu's lifetime,
  retaining share/sync behavior and avoiding success navigation on failure.
- [x] Make the red integration target pass, exercise duplicate confirmation and
  the real detail exit/autosave hooks, and verify rendered persistence errors.
- [x] Prove rollback/retry, delayed revocation success/failure and guarded
  navigation, including navigation during the exit animation.
- [x] Migrate Result callers and prove partial space-cleanup retry; run the six
  targeted suites: 38 tests passed. Remove only the lifetime fix in an isolated
  copy: regression fails; restore it: regression passes.

## Separate installation gate

Not performed or authorized here: native deletion from All Notes, theme,
subtheme, thread and after detachment on a candidate iPhone build. The
VirtualDom host is not a claim of whole-screen/native visual verification.

## Relevant Files

See `brief.md` for source anchors, observed evidence and its limitations.
