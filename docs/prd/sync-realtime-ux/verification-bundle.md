---
artifact: "docs/prd/sync-realtime-ux/"
stack: "rust / cargo"
generated: "2026-06-10"
ran_by: "user"
---

# Verification Bundle: Real-time sync experience and cross-device UI polish

> Each line states what it proves and the expected pass signal. Commands are stack-detected (cargo + Makefile).

## Safe checks (machine-local, no device needed)

| Command | Validates | Expected pass signal |
|---------|-----------|----------------------|
| `make check` | fmt + clippy on the whole diff | exit 0, no warnings |
| `cargo build --features mobile` | mobile feature compiles (default arm of the main.rs cfg alias) | exit 0 |
| `cargo build --features desktop` | desktop feature compiles (desktop arm of the cfg alias + dioxus::desktop re-export) | exit 0 |
| `cargo test` | all 205 existing tests + 3 new sync_data_version tests (C2, C3, C14, C15) | all green, 208 total |
| `make desktop-build` | Mac desktop app builds with the custom index config | exit 0 |

## Device / stateful (USER ONLY)

| Command / action | Validates | Warning |
|------------------|-----------|---------|
| `make all` | iOS build + URL-scheme injection + icon + install on iPhone | deploys to your device; check the injection log line ">> Registering URL scheme" |
| `make desktop-app` | Mac .app bundle | replaces the installed app |
| Device sweep: every view bottom (notes list, detail, chat, settings, sync) | C8, C9 | visual, home-indicator iPhone |
| Mac visual check: no added bottom gap | C10 | visual |
| Two-device test: create note on iPhone, sync, watch Mac update < 1 s (and reverse) | C1, C2 | needs both devices paired |
| Zero-change sync: press Sync now twice, watch for flicker | C3 | visual |
| Restore scenario (full-state reconcile) | C4 | visual smoothness |
| Edit collision x5: type on Mac without saving, push from iPhone, banner appears, text intact, tap reloads, save | C5, C6, C7 | count lost keystrokes (target 0) |
| QR spike: scan the Mac QR with the iPhone camera, app foregrounded AND app killed (cold start) | C11, C12 | record both outcomes in trace.md; cold-start failure routes to task 4.3 (in-app scanner) |
| Camera-permission-denied pairing via copy-paste | C13 | settings toggle on the iPhone |
| `flowflow://test` link tap (Notes app) | 4.1 spike observability | app should open; URI ignored by the pairing filter (not a pair URI) |

## Contract coverage

- C1 -> cargo test (signal plumbing) + two-device < 1 s trials (10x, stopwatch)
- C2 -> tests data_version_bumps_on_outbound_apply + data_version_bumps_on_served_apply + device both-directions test
- C3 -> test data_version_steady_on_zero_change_pass + visual zero-change check
- C4 -> restore-scenario device test (code: 400 ms write-on-change poll)
- C5/C6/C7 -> edit-collision device protocol above (code self-check PASS at detail.rs:179-208 post-fix)
- C8/C9/C10 -> device sweep + Mac visual check (code self-check PASS: viewport-fit=cover + env() utilities)
- C11/C12/C13 -> QR spike + copy-paste fallback tests above
- C14 -> make check + cargo build (both features) + cargo test + make all + make desktop-app
- C15 -> cargo test (sync suite untouched: protocol/, conflict.rs, gc.rs, vv.rs have zero diff)
- Uncovered by commands (manual only): C8, C9, C10 visuals; C11, C12 cold-start spike; < 1 s timing metric.

## Self-check results (read-only verifier, 2026-06-10)

13/13 contract items PASS at code level (file:line cites in trace.md). 4 MINOR findings, all fixed by the lead before this bundle:
1. detail.rs banner false-positive on unrelated syncs -> changed-check now precedes dirty-check.
2. chat view reload missed content-only edits -> ChatMsg derives PartialEq, full compare.
3. Makefile appstore target: url-scheme injection now ordered before icon, uniform with the other targets.
4. zero-change test: fixed sleeps replaced by an activity-settled wait.
