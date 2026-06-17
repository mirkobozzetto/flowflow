---
feature: Real-time sync experience and cross-device UI polish
slug: sync-realtime-ux
type: prd
status: ready
stepsCompleted: [0, 1, 2, 3, 4]
issues: [22, 23, 24]
---

# PRD - Real-time sync experience and cross-device UI polish

## Problem statement

Multi-device LAN sync (RFC 0004) shipped and works end to end at the data level:
pairing, encrypted bidirectional sync, merge, tombstones, GC, full-state
reconcile, all validated on real devices (iPhone <-> Mac). But the EXPERIENCE
around it is unfinished. Three gaps, observed during real device testing and
tracked as GitHub issues:

1. **No real-time refresh (#22).** After an inbound sync the data is in SQLite
   but the UI keeps showing stale state until the app is quit and relaunched.
   The user watched 25 notes land in the Mac database while the Mac window
   showed nothing new. The SyncEngine already exposes `SyncActivity`
   (Idle / Syncing / Done{applied} / Error) and the UI already polls it for the
   activity indicator; nothing makes the data views react.

2. **Bottom-of-screen cut on iPhone (#23).** On home-indicator iPhones the last
   line of content and bottom controls are clipped or covered by a white band.
   The bottom safe-area inset is not honored by the root layout. Root cause is
   verified: the default Dioxus 0.7.7 viewport meta lacks `viewport-fit=cover`,
   so `env(safe-area-inset-*)` evaluates to 0 inside the WKWebView, and the one
   existing safe-area padding in the app is a silent no-op.

3. **QR pairing does not work (#24).** Scanning the pairing QR with the iPhone
   camera fails ("no usable data found") because the `flowflow://` URL scheme
   is not registered. Copy-pasting the URI is the only working path, which is
   not an acceptable pairing experience between a Mac and an iPhone.

Why now: sync is the headline feature of the next release. Shipping it with a
stale UI, a clipped layout, and broken QR pairing undermines the feature users
will judge first.

## Goals

- A change synced from the other device is visible on screen in under ~1 s,
  with no app restart and no manual action, on both iOS and macOS.
- In-progress local edits are never silently overwritten by an inbound sync
  (zero-data-loss prime directive extends to screen state).
- On home-indicator iPhones, all bottom content and controls are fully visible
  above the home indicator in every view.
- Pairing two devices requires scanning the QR code and confirming, with no
  manual copy-paste of the URI.

## Non-goals / Out-of-scope

- Full desktop UX / responsive overhaul (issue #25, separate PRD).
- README slimming, docs index, landing page (issue #26 and follow-ups).
- Audio file sync (descoped from sync v1, unchanged).
- Android support.
- Multi-user collaboration or any third-party backend.
- Push-style instant sync between devices (sync triggers stay as shipped in
  RFC 0004; this PRD only makes the UI react to what sync already applies).

## User stories

### US1 - Live refresh after inbound sync
As a FlowFlow user with two paired devices, I want data synced from the other
device to appear on my screen immediately, so that I never wonder whether sync
worked or have to restart the app.

Acceptance criteria:
- Given the app is open on any data view (notes list, sidebar folders and
  conversations, note detail, chat, reminders), when an inbound sync applies at
  least one change, then the visible view reflects the new data in under ~1 s.
- Given the inbound change arrived through the background listener (the other
  device pushed), when it is applied, then the refresh happens exactly as it
  does for a user-triggered sync (both paths covered).
- Given a sync pass applies zero changes, when it completes, then no visible
  refresh or flicker occurs.
- Given a full-state reconcile applies many rows in bursts, when it runs, then
  the UI refreshes smoothly (no refresh storm, no frozen UI).

### US2 - Never clobber an edit in progress
As a user typing in a note while an inbound sync updates that same note, I want
to be warned instead of having my editor replaced, so that I never lose what I
am typing.

Acceptance criteria:
- Given the open note detail has NO unsaved local edits, when an inbound sync
  updates that note, then the detail reloads silently with the new content.
- Given the open note detail HAS unsaved local edits, when an inbound sync
  updates that note, then a non-blocking banner appears ("Note updated from
  another device"), the editor content is untouched, and tapping the banner
  reloads the merged note.
- Given the banner is shown and the user keeps typing and saves, when the save
  completes, then the local edit is persisted through the normal merge path and
  no keystrokes are lost.

### US3 - Bottom safe-area respected on iPhone
As an iPhone user, I want every view to keep its bottom content above the home
indicator, so that nothing is clipped or hidden.

Acceptance criteria:
- Given any view (notes list, note detail, chat, settings, sync screen), when
  displayed on a home-indicator iPhone, then the last line of content and all
  bottom controls are fully visible and tappable above the home indicator.
- Given the on-screen keyboard opens, when it closes, then no white band or
  clipped zone remains at the bottom.
- Given the same build runs on macOS desktop, when displayed, then the layout
  is unchanged (no spurious bottom padding on devices without insets).

### US4 - Pair by scanning the QR code
As a user pairing my iPhone with my Mac, I want to scan the QR code shown on
the Mac and confirm, so that I never copy-paste a pairing URI by hand.

Acceptance criteria:
- Given the Mac shows the pairing QR, when the iPhone scans it (system camera
  via the registered `flowflow://` scheme, or the in-app scanner if the scheme
  path fails the spike), then the iOS app opens or foregrounds with the pairing
  screen prefilled and pairing completes after one confirmation.
- Given the app is cold (not running), when the QR is scanned, then the pairing
  data still reaches the pairing screen (cold-start delivery verified or the
  in-app scanner covers it).
- Given the scan path fails for any reason, when the user falls back, then the
  existing copy-paste path still works (kept as a fallback, never removed).

## Settled decisions

These were the three open points; all are settled with verified evidence
(Exa research, 2026-06-10).

1. **Refresh mechanism: one global data-version signal, all data views react.**
   A single reactive version counter bumped after any inbound apply with
   `applied > 0` (manual sync button AND background listener path, including
   post-reconcile), debounced so reconcile bursts coalesce. All data views
   (list, sidebar, detail, chat, reminders) subscribe. Per-entity targeted
   invalidation is rejected for v1: more surface for staleness bugs, no
   measured need at FlowFlow's data scale. The open note detail applies the
   US2 smart rule (silent reload vs banner) on top of the global signal.

2. **QR pairing: register the `flowflow://` scheme, in-app scanner as the
   committed fallback.** Verified: Dioxus 0.7 runs on tao; the lockfile has
   tao 0.34.8 which implements `application:openURL:options:` on iOS and emits
   `Event::Opened { urls }` (tauri-apps/tao commit ad652e5), and dioxus-desktop
   0.7.7 forwards every tao event to app-level handlers before its own match
   (`app.tick()` -> `event_handlers.apply_event`), so the app can receive the
   URL. Scheme registration goes through `CFBundleURLTypes` in Info.plist
   (post-build injection, same pattern as the existing icon injection). One
   spike remains: cold-start delivery when the app is not running. If the spike
   fails, the v1 path becomes the in-app QR scanner on the iPhone pairing
   screen (camera via AVFoundation FFI); the scheme stays registered as a
   nice-to-have. Copy-paste remains as last-resort fallback either way.

3. **Safe-area: pure CSS via `env(safe-area-inset-*)`, enabled by fixing the
   viewport meta.** Verified: the default dioxus-desktop 0.7.7 index.html
   viewport meta lacks `viewport-fit=cover`, so the WKWebView reports inset 0.
   Dioxus exposes `Config::with_custom_index` to control the full index.html.
   With `viewport-fit=cover` set, `env(safe-area-inset-bottom)` works in pure
   CSS and applies uniformly; macOS reports 0 insets so desktop is unaffected.
   Per-platform Rust-side padding adjustments are rejected: CSS env() is the
   platform-native mechanism and needs no platform branching.

## Success metrics

- Inbound change visible on screen in < 1 s after apply completes, measured on
  real iPhone and Mac (manual stopwatch over 10 trials, all < 1 s).
- 0 lost keystrokes across 5 deliberate edit-during-sync collisions on device.
- 0 clipped or hidden bottom controls across all views on a home-indicator
  iPhone (visual sweep checklist, every view passes).
- Pairing two devices via QR completes in < 30 s with 0 manual copy-paste.
- 0 regressions: full existing test suite (205 tests) still passes; sync
  invariants (zero data loss, convergence) untouched.
- All acceptance criteria validated on real devices (iPhone + Mac), not only
  simulator.

## Constraints and assumptions

- 100% Rust, zero JS/TS. UI is Dioxus 0.7 (tao 0.34.8 / wry 0.53.5 locked).
- Targets: iOS (primary) + macOS desktop app. Behavior must be correct on both.
- Zero data loss; any migration is non-destructive.
- Must not break RFC 0004 sync (protocol, merge, GC) or the RAG pipeline.
- No third-party backend; everything stays local / LAN P2P.
- SyncEngine `SyncActivity` polling already exists and stays the integration
  surface for the activity indicator; the refresh signal is additive.
- Existing Makefile post-build plist injection is available for Info.plist
  changes (used today for the icon).

## Open questions

- Cold-start open-URL delivery: does tao's `Event::Opened` fire when the app is
  launched (not just foregrounded) by a `flowflow://` URL on iOS? Spike first;
  the in-app scanner decision already covers a negative outcome.
- Banner copy and placement for US2 (top of detail vs above keyboard): decide
  during implementation with on-device feel; not blocking.
- Whether the desktop Mac app needs the same banner rule (concurrent edit on
  Mac while iPhone pushes): assumed yes for symmetry, confirm during testing.
