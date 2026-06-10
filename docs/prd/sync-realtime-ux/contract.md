---
artifact: "docs/prd/sync-realtime-ux/"
artifact_kind: "prd"
locked: "2026-06-10"
---

# Definition of Done: Real-time sync experience and cross-device UI polish

> Immutable target. Every item below is a concrete, checkable condition the final verification bundle validates against. Requirement changes get a NEW entry; never silently rewrite an existing line.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | Given any data view open, when an inbound sync applies >= 1 change, then the view reflects the new data in under ~1 s (no restart, no manual action) | prd.md US1 | device test: 10 timed trials, both directions |
| C2 | Given the inbound change arrived via the background listener, when applied, then the refresh happens exactly as for a user-triggered sync | prd.md US1 | integration test on the three bump paths + device test |
| C3 | Given a sync pass applies zero changes, when it completes, then no visible refresh or flicker occurs | prd.md US1 | test asserting no bump on zero-change pass + visual check |
| C4 | Given a full-state reconcile applies many rows in bursts, when it runs, then the UI refreshes smoothly (debounced, no freeze) | prd.md US1 | restore-scenario device test |
| C5 | Given the open note detail has NO unsaved edits, when an inbound sync updates that note, then the detail reloads silently | prd.md US2 | device test: edit on iPhone, Mac detail updates in place |
| C6 | Given the open note detail HAS unsaved edits, when an inbound sync updates that note, then a non-blocking banner appears, editor untouched, tap reloads the merged note | prd.md US2 | device test: typed text intact, banner tap shows merged content |
| C7 | Given the banner is shown and the user keeps typing and saves, when the save completes, then zero keystrokes are lost (normal merge path) | prd.md US2 | 5 deliberate collision tests, 0 lost characters |
| C8 | Given any view on a home-indicator iPhone, when displayed, then bottom content and controls are fully visible and tappable above the home indicator | prd.md US3 | visual sweep checklist, every view |
| C9 | Given the on-screen keyboard closes, when it does, then no white band or clipped zone remains at the bottom | prd.md US3 | device test after keyboard close |
| C10 | Given the same build on macOS, when displayed, then the layout is unchanged (no spurious bottom padding) | prd.md US3 | desktop visual check, all views |
| C11 | Given the Mac shows the pairing QR, when the iPhone scans it, then the app opens/foregrounds with the pairing screen prefilled and pairing completes after one confirmation | prd.md US4 | full pairing Mac <-> iPhone < 30 s, zero copy-paste |
| C12 | Given the app is cold, when the QR is scanned, then the pairing data still reaches the pairing screen (cold-start verified or in-app scanner covers it) | prd.md US4 | cold-start spike result recorded in trace |
| C13 | Given the scan path fails, when the user falls back, then copy-paste pairing still works (never removed) | prd.md US4 | camera-permission-denied pairing test |
| C14 | Full existing test suite (205 tests) passes; fmt + clippy clean; make all and make desktop-app build and install | prd.md success metrics | cargo test, make check, make all, make desktop-app all exit 0 |
| C15 | Sync invariants (zero data loss, convergence, RFC 0004 protocol/merge/GC) untouched | prd.md constraints | existing sync test suite green, no protocol file regression |

## Out of scope (never build)

- Full desktop UX / responsive overhaul (issue #25, separate PRD)
- README slimming, docs index, landing page (issue #26)
- Audio file sync (descoped from sync v1)
- Android support
- Multi-user collaboration or any third-party backend
- Push-style instant sync (sync triggers stay as shipped in RFC 0004)

## Edit scope

- src/main.rs (launch config: custom index.html, app-level Event::Opened handler)
- src/ui/mod.rs (root layout, keyboard handler)
- src/ui/state.rs (data-version signal, pairing prefill state)
- src/ui/note_list.rs, src/ui/sidebar.rs, src/ui/note_detail.rs, src/ui/chat.rs (signal subscription + safe-area sweep)
- src/services/sync/engine.rs (bump points: manual pass, served session, post-reconcile)
- pairing module (URI parsing reuse) + pairing/sync screen UI
- src/platform/ios.rs (AVFoundation FFI only if the in-app scanner path is needed)
- Makefile + scripts/ (CFBundleURLTypes post-build plist injection)
- tailwind.css (safe-area utilities)
- tests/ (refresh-signal assertions)
