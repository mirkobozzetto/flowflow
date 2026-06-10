---
artifact: "docs/prd/sync-realtime-ux/"
artifact_kind: "prd"
engine_tier: "teams"
stepsCompleted: [0, 1, 2, 3, 4, 5, 6]
final_status: "shipped"
updated: "2026-06-10"
---

# Trace Ledger: Real-time sync experience and cross-device UI polish

> Single source of truth for progress. A fresh session reads ONLY this file to resume. One row per task/T-id.

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Notes |
|------|---------------|--------|---------------|--------|-------|
| 1.0 safe-area foundation | C8 C9 C10 | done (code) | src/main.rs, tailwind.css, src/ui/mod.rs, src/ui/note_list.rs, src/ui/notes/detail.rs, src/ui/chat/view.rs, src/ui/fab.rs, src/ui/attachment_modal.rs | teams (worker-a) | custom prod-style index.html with viewport-fit=cover via LaunchBuilder::with_cfg + client! (dioxus 0.7.9 API verified in vendored sources); safe-pb-* env() utilities replace fixed paddings; .keyboard-aware rule covers chat input + recording bars; attachment_modal safe-py-3 added by lead (scope-edge, logged); device sweep pending user |
| 2.0 real-time refresh | C1 C2 C3 C4 | done (code) | src/services/sync/engine.rs, src/ui/state.rs, src/ui/mod.rs, src/ui/sidebar/conversations.rs, src/ui/chat/view.rs, tests/sync_data_version_test.rs (new) | teams (worker-b) | data_version AtomicU64 on SyncEngine, bumped in serve_loop (~209) and run_pass (~349) only when applied>0; full-state reconcile is a session MODE so it flows through those 2 points; 400ms UI poll writes sync_data_version + bumps notes/folders/attachments versions only on change (debounce + C3); 3 tests written NOT run |
| 3.0 smart open-detail | C5 C6 C7 | done (code) | src/ui/notes/detail.rs | teams (worker-b) | base_title/base_content/base_tags baselines + updated_from_peer flag; effect re-runs only on sync_data_version; not dirty = silent DB reload + baseline advance; dirty = banner, editor signals untouched; tap reloads + clears; save path byte-for-byte unchanged (C7); banner text moved to .ftl by lead (note-updated-from-peer en+fr) |
| 4.0 QR pairing | C11 C12 C13 | done (code) | src/services/sync/deeplink.rs (new), src/services/sync/mod.rs, src/main.rs, src/ui/sync/pairing.rs, src/ui/sync/mod.rs, scripts/inject-url-scheme.sh (new), Makefile | teams (worker-c) | with_custom_event_handler (dioxus-desktop 0.7.9 config.rs:216) matches tao Event::Opened (tao 0.34.8 event.rs:137), flowflow:// pushed to deeplink static; pairing screen polls take() 400ms, prefills join_uri, one-tap confirm via existing join_pairing; plist injection idempotent, ordered before icon/codesign in all/ddev-build/deploy/appstore; copy-paste path untouched; 4.3 scanner NOT built (contingent on device spike); ui/mod.rs nav snippet pending lead integration after Unit B |
| 5.0 device validation | C14 C15 | todo | - | user | verification bundle, user-run on real devices |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-04 | scope-edge | proceeded (logged) | attachment_modal.rs not in declared edit scope but US3 says all views; one-line safe-py-3 on the bottom-sheet scroll region, applied by lead |
| step-04 | quality fix | proceeded (logged) | worker-c used 3 hardcoded lang helpers in pairing.rs; lead migrated them to the fluent .ftl pattern (sync-scanned-title, sync-scanned-hint, sync-confirm-pairing in en.ftl + fr.ftl), helpers removed |
| step-04 | quality fix | proceeded (logged) | worker-b banner literal in detail.rs moved to t(&lang, "note-updated-from-peer") with en+fr .ftl keys |
| step-04 | integration | done by lead | deep-link navigation folded into the existing 400ms poll loop in src/ui/mod.rs: pending_pairing_uri_exists() + view != SyncPairing -> previous_view saved, view set to SyncPairing (worker-c snippet, single timer instead of a third loop) |

## Verify (step-05, 2026-06-10)

- creator-verifier (read-only): 13/13 contract items PASS at code level. Key cites: engine.rs:211 + 351 (both bump paths, applied>0 guarded), ui/mod.rs:148-166 (400ms write-on-change poll), detail.rs:179-208 + 714-732 (smart rule + banner), main.rs:1-12 + 30-45 (viewport-fit=cover index + Event::Opened handler), deeplink.rs peek/take split, pairing.rs:29-39 prefill + one-tap confirm, Makefile 4 targets wired.
- 4 MINOR defects found, all fixed by lead: banner false-positive ordering (detail.rs), ChatMsg PartialEq content compare (chat), appstore target injection order (Makefile), zero-change test settle wait (tests).
- 1.0/2.0/3.0/4.0 done at code level; 5.0 = user-run verification bundle (verification-bundle.md). Device spike 4.1 (cold-start Event::Opened) decides whether 4.3 (in-app scanner) is needed.

## Device feedback round 1 (2026-06-10)

- Bottom OK on iPhone; TopBar (burger + chat icon) under the Dynamic Island: viewport-fit=cover also removes the TOP inset. Fix: safe-pt utility (padding-top: env(safe-area-inset-top)) on the app column (ui/mod.rs) and the sidebar drawer (sidebar/mod.rs). Rebuilt + reinstalled (make all exit 0).

## Safe checks run by ship (standing user authorization, 2026-06-10)

- make format + make check (fmt --check + clippy --features mobile): exit 0
- cargo build --features desktop: exit 0 (both cfg arms of main.rs compile)
- cargo test: 208 passed (205 existing + 3 new), 11 ignored (pre-existing key-gated), 0 failed
- make desktop-build: Flowflow.app built (debug/macos)
- make all: device build + inject-url-scheme + inject-icon + re-sign + install, exit 0
- PlistBuddy read-back: CFBundleURLTypes/flowflow present in the installed app's Info.plist

## HALT events

- none
