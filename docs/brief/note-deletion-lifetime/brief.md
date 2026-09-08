---
type: brief
slug: note-deletion-lifetime
title: Reliable note deletion from every entry point
status: ready
created: 2026-09-08
---

# Reliable note deletion from every entry point

## Problem

Mirko reports that deletion fails from themes/subthemes and leaves notes in
threads, including after detaching a note, while All Notes deletion works.
The initial request required diagnosis and integration evidence before a fix.
Mirko subsequently requested continuation through a tested correction. The
local source is now corrected and all 38 targeted tests pass; nothing was
installed or published. See ../../proposals/0008-note-deletion-lifetime/VALIDATION.md.

## Original pre-fix evidence

Integration tests against the current local checkout use the real menu source,
Dioxus VirtualDom click events and temporary SQLite databases. Global vector
storage is redirected to the same temporary directory. No personal data is used.

Observed results:

- List menu, All Notes: note deleted.
- List menu, selected theme: note deleted.
- Open-note menu, All Notes: note remains after confirmation.
- Open-note menu, selected theme: note remains after confirmation.
- Control retaining the open-note menu mounted: note deleted.
- SQLite deletion: passes for no folder, theme/thread, subtheme/thread, and
  theme/subtheme after detaching from the thread. Folder links disappear and
  the other thread member survives.

Cause demonstrated in this checkout: NoteMenu sets show_note_menu=false and
spawns deletion in its own scope. NoteDetail conditionally mounts that menu.
Removing the component cancels its task before deletion. NoteRowMenu stays
mounted at the app root, so its task survives. The selected folder alone is
not the discriminating factor in the reproduced failure.

The exact gestures and installed iPhone build have not been inspected. This
proves a matching code defect, not that every observed instance has this cause.
GitNexus was stale by two commits and impact traversal failed on staticGated;
current source and execution, not an empty graph result, support this finding.
Existing unrelated local persistence/sync changes remain untouched.

## Acceptance criteria

- Confirmed deletion from an open note survives menu closure and navigation.
- The same note disappears from All Notes, its theme/subtheme and its thread;
  unrelated notes and the thread's other members remain intact.
- Detaching a note from a thread does not prevent subsequent deletion.
- List-menu deletion remains functional.
- Cancellation preserves the note; failed persistence must not be presented
  as successful deletion.
- Existing share revocation and sync scheduling are preserved; a pending
  network request must not be canceled by destruction of the menu owner.

## Success metrics

The diagnostic integration target becomes green without weakening its deletion
assertions. The corresponding native UI scenarios pass on the candidate build.

## Out-of-scope

Folder/thread identifier migration, speculative SQLite repairs, unrelated sync
work, personal database changes, release, installation or publication without
separate approval.

## Original source anchors

- `src/ui/notes/menu.rs:175-188`: confirmation and scoped deletion task.
- `src/ui/notes/detail/mod.rs:118-131`: exit triggered before deletion completes.
- `src/ui/notes/detail/mod.rs:262-268`: conditional menu lifetime.
- `src/ui/notes/row_menu.rs:143-155`: working list-menu deletion.
- `src/ui/app/router.rs:58`: persistent list-menu owner.
- `src/application/note_persistence.rs:55-76`: shared deletion use case.
- `tests/note_delete_integration_test.rs`: executed diagnostic coverage.

## Verification

```sh
PATH="$HOME/.cargo/bin:$PATH" cargo test -j 8 \
  --test note_delete_integration_test -- --nocapture --test-threads=1
```

Before any fix: 1 passed, 1 failed. The failure asserts the expected behavior;
the mounted control passes. With the full correction: 2 passed, 0 failed.
Removing only the lifetime fix from a temporary source copy makes the same
regression fail; restoring the production import returns it to green.
Five additional thread/purge/space/sync suites pass, for 38 tests overall.
Native whole-screen/device verification remains a separate installation gate.
