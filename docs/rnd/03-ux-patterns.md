# 03 — UX Patterns

Mobile-first UX patterns applicable to FlowFlow. All patterns evaluated for feasibility in Dioxus 0.7 WKWebView (HTML/CSS only, no native UIKit).

## Constraints

- WKWebView = no native iOS APIs without objc2 bridge
- No native gestures (swipe-to-delete, peek-and-pop, force-touch) — only JS touch events
- No native haptics without objc2 (basic `navigator.vibrate()` available but limited)
- Tailwind CSS V4 already in place, 14 SVG icon components
- Slide transitions (150ms) already implemented

## Tier 1 — High Impact, Low Effort

### 1. Pinned Notes (S)
- Universal pattern (Bear, Apple Notes, Simplenote, Keep)
- "Pinned" section at top of list, pin icon on card
- `pinned: bool` + `pinned_at: timestamp` in DB
- Sort: `ORDER BY pinned DESC, modified_at DESC`

### 2. Toast / Snackbar with Undo (S)
- Gmail pattern: immediate action + 5-10s toast with "Undo" button
- Better than confirmation dialogs (doesn't second-guess the user)
- Dioxus component controlled by AppState signal, CSS slide-up animation
- Soft delete: `deleted_at` column instead of physical DELETE

### 3. Daily Timeline Grouping (S)
- Group notes by date: "Today", "Yesterday", "8 May 2026"
- Sticky section headers via CSS `position: sticky`
- Natural for voice notes (inherently timestamped)
- Inspired by Craft Daily Notes and Agenda timeline

### 4. Empty States (S)
- Every empty view = teaching opportunity
- Structure: icon/illustration + short title + help text + primary CTA
- Cases: no notes, no search results, empty folder, no tags
- Example: "No notes yet" + "Tap the mic to start recording"

### 5. Long-Press Action Sheet (S/M)
- Apple Notes / Bear pattern: long press on card → bottom sheet with actions
- More accessible than swipe (always discoverable)
- Implementation: `pointerdown` + `setTimeout(500)` in Dioxus event handler
- Actions: Pin, Move to folder, Share, Delete
- Basic vibration: `navigator.vibrate(10)` on iOS WKWebView (limited but works)

## Tier 2 — Medium Effort, Strong ROI

### 6. Sort & Filter Bottom Sheet (M)
- Button in TopBar → bottom sheet with sort options
- Sort by: date created, date modified, title alphabetical
- Persist per folder (store in AppState or SQLite)
- Bottom sheet = dominant mobile pattern (2025-2026)

### 7. Persistent Search Bar (M)
- Search bar fixed at top of notes list
- Debounce 200ms, instant results via SQLite FTS5
- Highlight matching text in results (SQLite `snippet()`)
- Recent searches shown before typing (3-5 items)
- Clear button to reset search

### 8. Multi-Select Mode (M)
- Long press on card → enters selection mode
- Action bar at bottom: Pin, Move to folder, Delete
- Header becomes "X selected · Cancel"
- Critical for batch operations (move 10 notes to a folder)

### 9. Badges / Visual Indicators on NoteCard (S)
- Small icons or dots for quick scanning:
  - Mic icon: note has audio recording
  - Paperclip: note has attachments (photos, files)
  - Spinner: transcription in progress
  - AI badge: AI processing title/tags
  - Pin icon: note is pinned
- 8x8px colored dots or 16px icons

### 10. Bottom Sheet Component (M)
- Reusable component for: sort/filter, action sheet, folder picker
- Slide-up from bottom, dark overlay, close on tap outside
- CSS animation consistent with existing slide transitions
- Touch-friendly: minimum 44px tap targets

## Tier 3 — Lower Priority or Conditional

### 11. Basic Markdown Rendering (M/L)
- Support: `**bold**`, `*italic*`, `` `code` ``, `- lists`, `# headings`, `[ ] checkboxes`
- Rust crate `pulldown-cmark` for parsing → render to HTML in Dioxus
- Or: use Dioxus `eval()` to run a JS markdown parser (marked.js ~30KB)
- First wave: headings, bold, italic, lists only. No tables/LaTeX.

### 12. Web Share API (S)
- `navigator.share({title, text, files})` opens iOS native share sheet
- Works in WKWebView on iOS 12+, requires user gesture
- Export note text to Messages, Mail, Notes, etc.
- Can share WAV audio files too

### 13. Basic Haptic Feedback (S — limited)
- `navigator.vibrate(10)` works in WKWebView but NOT the fine Taptic Engine
- Use sparingly: recording start/stop, pin toggle, delete confirmation
- Fallback: silent on unsupported (always test on real device)

### 14. Pull-to-Refresh (M)
- Only useful if there's async data to refresh (future: sync, transcription status)
- Implementation: touchstart/touchmove on scrollTop=0, threshold ~60px
- Not needed now — all data is local SQLite

### 15. FAB Enhancement (S)
- Current FAB: single tap → new note
- Enhancement: long-press FAB → radial menu: "Voice note", "Text note", "Photo note" (future)
- Or: keep simple (current approach is clean)

## Not Feasible Without Native Swift Bridge

These require a Swift wrapper via FFI, not achievable in pure Dioxus WKWebView:

| Feature | iOS API Required | Effort |
|---------|-----------------|--------|
| Lock Screen widget | WidgetKit extension | High |
| Action Button mapping | App Intents | High |
| Control Center button | ControlWidget (iOS 18) | High |
| Siri Shortcuts | Intents framework | Medium |
| Dynamic Island | Live Activities (advanced) | Medium |
| True Taptic Engine haptics | UIImpactFeedbackGenerator | Low (objc2) |
| Apple Pencil drawing | PencilKit | High |
| Background recording indicator | Live Activities | Medium |

These are on the "port to SwiftUI" roadmap, not the Dioxus roadmap.

## Recommended Implementation Order

1. **Sprint 1** (1-2 days): Pin notes + Toast undo + Empty states — foundational UX
2. **Sprint 2** (1 day): Daily timeline + Badges — visual polish
3. **Sprint 3** (2 days): Long-press action sheet + Bottom sheet component — interaction power
4. **Sprint 4** (2 days): Sort/filter + Search bar (FTS5) — scale management
5. **Sprint 5** (1-2 days): Multi-select — batch operations
6. **Sprint 6** (2-3 days): Markdown rendering — power user feature
