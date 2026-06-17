---
artifact: "docs/rfcs/0001-data-backup-export/RFC.md"
stack: "rust / cargo"
generated: "2026-06-11"
ran_by: "claude (per Mirko's standing rule: run make/cargo yourself) + Mirko (device)"
---

# Verification Bundle: Backup, export & restore (RFC 0001 rev2)

> Automated checks already ran green during the ship (Mirko's standing rule overrides the ship default). What remains is the DEVICE validation, which only Mirko can do.

## Already run (all green, 2026-06-11)

| Command | Validates | Result |
|---------|-----------|--------|
| `cargo test` | C1, C2, C3, C5, C6, C7, C9 (54 backup/restore tests within the 249) | 249 passed, 11 ignored |
| `make check` (fmt + clippy --features mobile) | style/static | exit 0, zero warnings |
| `IPHONEOS_DEPLOYMENT_TARGET=16.0 cargo check --target aarch64-apple-ios` | iOS cross-compile (share.rs, picker, sync_ffi) | exit 0 |

## Device / desktop manual validation (Mirko)

| Step | Validates | Expected |
|------|-----------|----------|
| `make all` then on iPhone: Settings -> Sauvegarde -> Exporter | C4 iOS | share sheet opens with `flowflow-backup-*.ffbak.zip`; AirDrop to Mac works |
| Inspect the archive on Mac (`unzip -l`, open db/flowflow.db) | C1/C2 | manifest.json + db + audio only; no API key, no sync_psk_, no privkey in settings |
| `make desktop-app`: Exporter | C4 desktop | rfd save dialog, file revealed in Finder |
| iPhone: Importer the archive -> confirm | C5 | summary + warning shown; after confirm, blocking "Redémarrage requis" screen; creating notes impossible |
| Home + reopen the app (NO force quit) | C5 (BLOCKER #3) | still locked on the restart screen, sync indicator silent |
| Force quit + relaunch | C6 | data restored; notes/audio/tags identical; consent screen shown again (keys re-entry) |
| Search a note right after relaunch | C8 | rebuild banner visible until reconcile ends; search complete afterwards |
| Settings -> Ré-appairer with the Mac | C7 | first attempt refused; confirm rebind; full-state session; post-restore notes intact on both |
| Import a corrupted file (truncate the zip) | C5 | clean refusal, data intact |
| Export of a realistic volume (Q1: target to confirm) | perf < 30 s | export completes under 30 s |

## Contract coverage

- C1 scrub consts + manifest -> `cargo test backup::` (manifest_tests, scrub_const_tests)
- C2 snapshot + scrub + gate -> `cargo test backup::` (snapshot_tests, capture-then-scan)
- C3 audio collection + explicit zip -> `cargo test backup::` (archive_tests)
- C4 share/save UX -> device + desktop manual (above)
- C5 import validation + lock -> `cargo test backup::` (import_tests) + device manual
- C6 swap + fault injection -> `cargo test backup::` (swap_tests, 10 states)
- C7 protocol -> `cargo test --test backup_restore_test` (4 sync scenarios) + existing RFC 0004 suite
- C8 Settings UI -> device manual (above)
- C9 suite -> `cargo test` full
- Uncovered criteria (manual only): C4, C8, perf Q1
