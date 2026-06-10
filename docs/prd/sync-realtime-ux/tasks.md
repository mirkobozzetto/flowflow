---
feature: Real-time sync experience and cross-device UI polish
slug: sync-realtime-ux
type: tasks
source_prd: docs/prd/sync-realtime-ux/prd.md
stepsCompleted: [0, 1, 2, 3]
---

> ⚠️ Do NOT implement. This is the derived task list. Run `ship` (or the implementer) to execute.

## Relevant Files

- `src/main.rs` - app launch; custom launch config entry point for the viewport fix
- `src/ui/mod.rs` - root layout (h-screen, fixed bottom padding); keyboard handler
- `src/ui/state.rs` - AppState; home of the new data-version signal
- `src/ui/note_list.rs`, `src/ui/sidebar.rs`, `src/ui/note_detail.rs`, `src/ui/chat.rs` - data views that must subscribe to the refresh signal
- `src/services/sync/engine.rs` - inbound apply completion points (manual pass, served session, reconcile) where the signal is bumped
- `src/services/sync/pairing.rs` (or equivalent) - pairing URI parsing reused by the deep link / scanner path
- `src/platform/ios.rs` - AVFoundation FFI home if the in-app scanner path is needed
- `Makefile` + `scripts/` - post-build Info.plist injection pattern (CFBundleURLTypes)
- `tailwind.css` - safe-area utility classes
- `tests/` - sync engine tests extended with refresh-signal assertions

## Tasks

- [ ] 1.0 Safe-area foundation: bottom of screen fully visible on iPhone _(PRD: US3, decision 3)_
  - [ ] 1.1 Enable real safe-area values: ship a custom index.html (viewport meta with `viewport-fit=cover`) through the launch config, identical behavior on iOS and desktop. Test: a debug element styled with `env(safe-area-inset-bottom)` shows a non-zero inset on the iPhone and zero on the Mac.
  - [ ] 1.2 Replace the fixed bottom paddings in the root layout with safe-area-aware spacing, and sweep every view (notes list, note detail, chat, settings, sync screen) for bottom clipping. Test: on the iPhone, in each view, the last line and bottom controls are fully visible and tappable above the home indicator; no white band after keyboard close.
  - [ ] 1.3 Desktop neutrality check. Test: same build on the Mac shows no added bottom gap in any view.

- [ ] 2.0 Real-time refresh after inbound sync _(PRD: US1, decision 1)_
  - [ ] 2.1 Introduce one global data-version signal in app state, bumped after every inbound apply with `applied > 0`: manual sync pass, background served session, and post-reconcile; debounced so reconcile bursts coalesce into one bump. Test: a unit/integration test asserts the bump fires on each of the three paths and not on a zero-change pass.
  - [ ] 2.2 Subscribe all data views (notes list, sidebar folders + conversations, chat, reminders) to the signal so they re-query on bump. Test: with both apps open, create a note on the iPhone, run sync; the Mac list and sidebar show it in under ~1 s without any click; repeat in the other direction.
  - [ ] 2.3 No-flicker and burst behavior. Test: a zero-change sync causes no visible refresh; a full-state reconcile (restore scenario) refreshes smoothly without freezing the UI.

- [ ] 3.0 Smart open-detail handling: never clobber an edit in progress _(PRD: US2)_
  - [ ] 3.1 Silent reload of the open note detail when it has no unsaved local edits and an inbound sync touched that note. Test: open the same note on both devices, edit on the iPhone, sync; the Mac detail updates in place in under ~1 s.
  - [ ] 3.2 Non-destructive banner when local unsaved edits exist: "Note updated from another device", editor untouched, tap reloads the merged note. Test: type in the note on the Mac without saving, push an edit from the iPhone; the banner appears, the typed text is intact, tapping the banner shows the merged content.
  - [ ] 3.3 Collision save path: saving after the banner goes through the normal merge with zero keystroke loss. Test: 5 deliberate edit-during-sync collisions on device, 0 lost characters (PRD metric).

- [ ] 4.0 QR pairing without copy-paste _(PRD: US4, decision 2)_
  - [ ] 4.1 Spike: register `CFBundleURLTypes` for `flowflow://` via the post-build plist injection and listen for the tao `Event::Opened { urls }` in an app-level event handler; verify delivery foregrounded AND cold-start on the real iPhone. Test: tapping a `flowflow://test` link opens the app and the URL reaches the handler in both states; result recorded in trace.
  - [ ] 4.2 If the spike passes: wire the received pairing URI into the pairing screen (prefill + one-tap confirm), reusing the existing URI parser; iOS camera scan of the Mac QR opens the app on the pairing screen. Test: full pairing Mac <-> iPhone via camera scan in under 30 s, zero copy-paste.
  - [ ] 4.3 If the spike fails: in-app QR scanner on the iPhone pairing screen (camera via AVFoundation FFI, permission prompt, scan -> prefill -> confirm). Test: same full-pairing test as 4.2 through the in-app scanner.
  - [ ] 4.4 Keep copy-paste as the always-available fallback and make the pairing screen present the paths clearly. Test: with camera permission denied, pairing still completes via copy-paste.

- [ ] 5.0 Device validation and regression pass _(PRD: success metrics)_
  - [ ] 5.1 Full regression: existing test suite (205 tests) green, fmt + clippy clean, `make all` and `make desktop-app` build and install. Test: all commands exit 0.
  - [ ] 5.2 Measured acceptance on real devices: 10 timed inbound-refresh trials all < 1 s, bottom-clipping sweep checklist all views pass, QR pairing < 30 s, 5 collision tests 0 loss. Test: each PRD success metric checked off with measured values recorded in the trace.
