---
proposal_id: "0009"
slug: "thread-note-context-menu"
title: "Menus des notes dans les threads et réinitialisation du déplacement"
status: Accepted
format: standard
created: "2026-09-08"
scope_path: "src/ui"
---
# 0009 : Menus des notes dans les threads et réinitialisation du déplacement

## 1. Résumé

Add a context menu to each local thread member note: Copy and Share only.
Support desktop right-click and long press, and mobile long press. Preserve
ordinary click/tap navigation. Reset note-menu subpages on every new opening,
including reopening the same note, without restarting the app.

Reuse the existing root-owned menu, clipboard helper and note-sharing flow.
Attach transient menu pages to the active menu selection instead of keeping
independent booleans alive after closure. Preserve the verified deletion fix.

This is a proposal only. No application code, test, database or build was
changed for this request. Implementation requires Mirko's explicit acceptance.

## 3. Problème et motivation

Mirko confirms the previous deletion correction works. He now reports that
choosing Move to, dismissing the popup and opening another note's menu leaves
the destination-folder screen visible. That observation is accepted as fact;
it was not rerun against his personal data.

Current source explains the retained state:

| Source | Observation |
| --- | --- |
| `notes/row_menu.rs:72-82` | Root component owns `moving` and `confirm_delete` signals; no active note returns empty RSX but does not destroy those signals. |
| `notes/row_menu.rs:118-153,203-207` | Move to sets `moving=true`; destination selection resets it, external dismissal does not. |
| `sidebar/mod.rs:89-109` | Outside dismissal only sets `app.row_menu=None`. |
| `keyboard/macos.rs:181-188` | Escape also clears the target, not the two local menu flags. |
| `notes/note_card.rs:66-119` | Right-click/hold sets a new note target without resetting the persistent subpage. |
| `thread/detail.rs:161-199` | ThreadNode has ordinary click navigation only, no context-menu or hold handlers. |
| `app/router.rs:58` | NoteRowMenu stays mounted at the app root. This lifetime is intentional. |

This is source-level diagnosis consistent with the report, not a claimed new
native reproduction. GitNexus query ran against FlowFlow; its index is two
commits stale. Impact for NoteRowMenu returned partial/UNKNOWN because
`staticGated` is missing. LSP and current text located callers, but some LSP
line numbers were stale. No graph-based all-clear is claimed.

Boundary: current dirty `dev` checkout at `09e34a5`; no open PR was returned.
Remote refs were not fetched, so no synchronization claim is made. Preserve
all prior deletion and unrelated sync/identity changes.

## 4. Objectifs et non-objectifs

- A menu belongs to exactly the note pressed, never its entire thread.
- Thread member menu: Copy and Share; no Move to and no new Delete entry.
- Existing delete actions in the open-note view remain available and unchanged.
- List/theme/subtheme menus retain their current actions, including Move to.
- Opening any note menu always starts at its action list, even after dismissing
  Move to or a deletion confirmation, and even when reopening the same note.
- Dismissal never moves/deletes a note or changes its folder/thread membership.
- Preserve the existing menu design, anchor positioning, backdrop and gestures.

Excluded: thread reordering, moving/detaching from the new menu, native OS share
sheets, a new sharing backend, persistence migrations, global overlay redesign,
changes to remote SharedThreadView, and automatic publication of personal notes.

## 5. Alternatives envisagées

| Option | Assessment |
| --- | --- |
| Add isolated resets to some close handlers | Small patch but misses alternate closure paths and leaves two independent sources of menu state. |
| Reset local flags through a reactive effect | Fewer changed lines, but reset timing remains separate from opening and must handle same-note/repeated openings. |
| Unmount/remount the whole menu on closure | Reject: risks cancelling its scoped move/delete tasks, recreating the earlier deletion defect. |
| Put the page in the active note-menu state | Recommended: closing removes target and page together; opening explicitly starts at Actions, while the root owner survives. |

## 6. Conception retenue

### État du menu

In `ui/state.rs`, replace `RowMenu::Note(String)` with a note variant carrying
`note_id` and a small `NoteMenuPage` enum: Actions, Move, ConfirmDelete.
Remove the corresponding persistent `moving` and `confirm_delete` signals
from NoteRowMenu; do not retain compatibility variants or parallel state.

Add `RowMenu::ThreadNote { note_id, thread_id }`. This variant has no move or
delete subpage. The origin is explicit, not guessed from a later global view.
Conversation, Folder and Space variants remain unchanged.

Transitions:

- List right-click/hold -> Note(id, Actions).
- Move to -> Note(same id, Move); no database write until a destination is chosen.
- Delete -> Note(same id, ConfirmDelete); existing confirmation logic remains.
- Any dismissal -> None, which also discards the page.
- Reopen same/different note -> Actions, never a remembered page.
- Thread right-click/hold -> ThreadNote(note id, thread id), Copy/Share only.

Migrate all constructors/matches in NoteCard, NotesList, NoteRowMenu and the
existing deletion integration target; expose the page type through the current
`ui/mod.rs` reexport pattern. Match selected-card highlighting by note ID,
not equality with the Actions page, so highlighting survives submenu changes.

Keep NoteRowMenu mounted at `app/router.rs:58`. Preserve the existing confirmed
move/delete execution owner, pending/error handling and post-commit ordering.
No application persistence or synchronization change is required.

### Gestes dans le thread

Extend ThreadNode's note-card surface using the existing NoteCard pointer
sequence: 450 ms hold, 10 CSS-pixel travel tolerance, cancellation on release,
leave or pointercancel, and suppression of the click produced by a completed
hold. Right-click prevents the native context menu and invalidates the timer.
Retain ordinary click/tap opening the note with previous_view set to the thread.

Use the existing pattern and shared threshold values, not a new gesture
framework or an unrelated gesture refactor. Only the selected note is targeted.
The root menu/backdrop stays outside the transformed thread container. Keep
menu actions above the backdrop and use the current selected-card treatment.
A scroll or short press must not publish, copy or open a menu accidentally.

### Copier

Reuse `clipboard::copy_text` and the current full `title + blank line + content`
payload from NoteRowMenu, not the truncated thread-card preview. Copy only the
selected note, close the menu, and remain in the thread. Copy is clipboard copy,
not creation of a duplicate note. Existing untitled-note localization remains.

### Partager

Reuse the current note-sharing behavior rather than introducing another flow:
close the menu, remember the source thread in previous_view, set share_request
to the selected note ID, then open that note's NoteDetail.

Its existing ShareSection consumes the request, displays the sharing section,
reuses an existing link or publishes this single note and copies its link.
`sharing::publish_note` sends one note (`sharing.rs:53-67`); never pass the
thread ID to its ShareSection or invoke publish_thread from this action.
Existing offline/error handling remains visible there. Back returns to the
source thread under the current navigation rules.

This navigation is an explicit approval point: Copy stays in the thread;
Share opens the note's existing sharing section. Sharing without leaving the
thread would need a different result/error surface and is not this proposal.
Reuse current English/French labels and menu styling; no new UI library.

## 7. Inconvénients et risques

- Preserve root lifetime: state reset must not undo the successful deletion fix.
- Pointer/native-context events can overlap; test one effective opening, no
  trailing navigation, and cancellation during scrolling on real platforms.
- Clipboard and WebView gestures cannot be proved by VirtualDom alone. Verify
  actual paste and gestures in the installed desktop/iPhone apps after approval.
- Sharing publishes content by link, not an OS share sheet. Use synthetic data
  with a loopback backend for automated checks; no real personal-note publication.
- ThreadDetail currently removes threads with fewer than two members on unmount
  (`thread/detail.rs:22-31`). Reusing note navigation preserves this existing
  rule, not a new guarantee of returning to a singleton thread. Cover normal
  two-or-more-member threads and the existing singleton fallback separately.
- Pending deletion/error state is operational state, not a menu subpage. Do not
  clear an in-flight deletion merely because another menu opens.

## 9. Recommandation et justification

Keep one root-owned menu and one explicit selection/page state. Reuse existing
copy/share operations and gesture semantics. This fixes the reported lifetime
mismatch and adds the requested two actions without touching note storage.

Acceptance requested for this exact scope, including the existing Share
navigation and the targeted verification below. No implementation is authorized
by drafting this document.

## 10. Plan d'implémentation

| ID | Title | Files | Depends on | Effort |
| --- | --- | --- | --- | --- |
| T1 | Prove stale-page transitions before the fix | Existing Dioxus integration harness and a bounded menu-state scenario | Approval | S |
| T2 | Move note-menu pages into the active selection | ui/state.rs; ui/mod.rs; notes/row_menu.rs; note_card.rs; note_list.rs; existing test constructors | T1 | S |
| T3 | Add thread-note gestures and Copy/Share routing | thread/detail.rs; notes/row_menu.rs; shared gesture threshold definitions | T2 | S |
| T4 | Verify state, targeting, gestures and deletion non-regression | Targeted Dioxus/SQLite tests and native checks | T3 | S |
| T5 | Install the usual test apps and hand off | make all; make desktop-app; this evidence record | T4; build/install approval | S |

Before source edits, refresh branch/dirty-path/worktree/PR context and symbol
references/impact. Preserve current local fixes. New targeted regression tests
are part of the proposed execution scope; none are written before acceptance.

### Critères de validation

| Scenario | Required result |
| --- | --- |
| A -> Move to -> outside dismissal -> B, then A | Each opening starts at Actions; no folder membership changes. |
| A -> Move to -> Escape -> same A | Actions immediately, no stale folder picker. |
| Dismiss deletion confirmation -> another note | Actions, no inherited confirmation and no deletion. |
| Choose a destination deliberately | Only the intended note moves; next opening starts at Actions. |
| Abandon Move to in list -> open thread member menu | Only Copy/Share; no leaked destinations or delete confirmation. |
| Desktop right-click or desktop/mobile hold on member B | Menu targets B; thread/detail is not opened by the release click. |
| Short click/tap; scroll beyond tolerance; cancelled hold | Ordinary navigation or scrolling, no accidental menu/action. |
| Copy a long note B | Actual pasted title/full content belongs to B; remain in thread. |
| Share B among different members | Only B's ID/content is published or reused; existing link/error UI is visible. |
| Return from the note after Share | Return to a surviving source thread; existing singleton fallback stays coherent. |
| Dismiss and reopen menus repeatedly | No blocked taps, stale subpage or duplicate action. |
| Existing deletion integration target | All original assertions still pass, including rollback/retry and navigation. |

Automated fixtures must use disposable SQLite/vector stores and synthetic
sharing data. Prove the new state regression fails before T2 and passes after
it; compile all migrated callsites and retain the existing deletion target.
Do not claim system clipboard or native touch correctness from a mock.

For Mirko's direct tests, after build/install approval run `make all` and
`make desktop-app` sequentially, never substitute debug-only bundle targets.
Use his installed applications and existing synchronized data, without reset
or an empty alternate profile. Verify actual iPhone installation despite any
masked script error, and the installed Mac binary; restart the desktop app.
Mirko chooses disposable notes for destructive/share checks. No push or release.
