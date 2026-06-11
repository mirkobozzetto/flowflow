---
artifact: "docs/rfcs/0001-data-backup-export/RFC.md"
artifact_kind: "rfc"
engine_tier: "solo"
stepsCompleted: [0, 1, 2, 3, 4, 5, 6]
final_status: "shipped (pending device validation, nothing committed)"
updated: "2026-06-11"
---

# Trace Ledger: Backup, export & restore (RFC 0001 rev2)

> Single source of truth for progress. A fresh session reads ONLY this file to resume. One row per task/T-id.

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Notes |
|------|---------------|--------|---------------|--------|-------|
| T1 | C1 | done | `settings_repo.rs`, `db/mod.rs`, `services/backup.rs` (new), `services/mod.rs` | solo | 4 scrub lists + is_excluded_from_backup; Manifest/Counts/Entry serde; current_schema_version() dynamic; 7 tests green |
| T2 | C2 | done | `backup.rs`, `reconcile.rs` | solo | create_scrubbed_snapshot (RO conn + VACUUM INTO + scrub MEMORY/secure_delete/VACUUM + no-sidecar), snapshot_counts, ensure_chunks_backfilled gate, is_backfilled helper; capture-then-scan unit test green (4 tests) |
| T3 | C3 | done | `backup.rs`, `Cargo.toml` (+crc32fast, already transitive via zip) | solo | audio_paths_from_snapshot, build_archive (explicit entries, streaming crc32, manifest last), missing WAV -> audio_missing; 3 tests green |
| T4 | C4 | done | `platform/ios/share.rs` (new), `platform/ios/mod.rs`, `backup.rs`, `Cargo.toml` (+rfd 0.15 desktop-only, +UIActivityViewController features) | solo | share_file (UIActivityViewController), export_archive (sweep + staging + zip), save_archive_dialog (rfd, Q2 default) + reveal Finder; iOS target check green |
| T5 | C5 | done | `backup.rs`, `engine.rs`, `transcription_manager.rs`, `sync_ffi.rs`, `ui/mod.rs`, `ui/restore_lock.rs` (new), locales | solo | validate_archive (RO, CRC, anti-tamper, floor>=10, stray-entry reject), stage_import (markers in staged db + fsync + rename), restore lock (engine+transcription gates, WillEnterForeground observer, lock screen); 9 import tests; full suite 234 green; iOS check green. Note: picker UTType stays "zip" (custom double ext needs Info.plist UTI; validation is the gate) |
| T6 | C6 | done | `backup.rs`, `main.rs`, `db/mod.rs` (raw_db_path), `vectordb.rs` (pub path), `ui/mod.rs` | solo | apply_pending_restore (state machine, re-validation via restore_state.json CRC, WAV CRC collisions + set-aside, checkpoint TRUNCATE, vectordb purge, single-file rename commit, rollback-or-abort), finalize_restore_bak 2-boot purge, orphan-cleanup gate; 10 swap fault-injection tests; full suite 244 green |
| T7 | C7 | done | `wire.rs`, `protocol/mod.rs` (v3), `session.rs`, `apply.rs`, `peers.rs`, `settings_repo.rs` | solo | Hello.restored + RestoredFloor msg (post-reseed, fixes floor remap race), per-peer done marks + global clear, authority exemption origin_seq>floor, HLC guard (dominated-but-newer -> Concurrent), confirmed rebind (one-shot authorize, row preserved, ack book cleared, rotation warning), sync_rebind_ prefix in scrub; suite 244 green; dedicated tests in T9 |
| T8 | C8 | done | `ui/settings.rs`, `ui/mod.rs`, `reconcile.rs` (running flag), locales | solo | Backup section (export/import, busy states), confirm with lineage warning (red for other-lineage), restore-error banner (take_restore_error), re-pair invite when restored_pending, index-rebuild banner (recovery window + reconcile_running), i18n FR/EN; re-consent forced by construction (ai_consent scrubbed); both targets compile |
| T9 | C9 | done | `tests/backup_restore_test.rs` (new), `apply.rs` (origin guard widened for full-state relay) | solo | 5 integration tests: full-pipeline restore + rebind + no-resurrection + post-restore creations survive; 3-device per-peer marker lifecycle; HLC guard (fresh edit wins, stale archived); rebind one-shot + ack book cleared + watermark preserved; round-trip over DIVERGENT pre-existing vectordb (BLOCKER #4 regression, real LanceDB). Found+fixed pre-existing RFC 0004 gap: origin guard refused third-party rows that push_full replays by design (only visible with 3 devices). Perf metric (<30s, Q1) deferred to device test. Suite: 249 tests green, clippy clean, iOS check green |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-04 T3 | new dependency | proceeded + logged | crc32fast 1.x (already transitive via zip, zero new code in tree) |
| step-04 T4 | new dependency | proceeded + logged | rfd 0.15 desktop-only (RFC-specified, Q2 default = save dialog) |
| step-04 T7 | scope edge | proceeded + logged | apply.rs origin guard widened to full-state any-origin: pre-existing RFC 0004 gap, push_full replays every row by design, watermark already filters on peer origin; without it any 3-device full-state session aborts |
| step-04 T5 | deviation | logged | picker keeps UTType "zip" (custom .ffbak.zip UTI needs Info.plist declaration; validation is the real gate) |
| step-04 T6 | design fix | logged | stage_import writes restore_state.json (post-marker CRC) because markers invalidate the manifest CRC for phase-2 re-validation |

## HALT events

- none

## Open questions surfaced (RFC §8)

- Q1 export volume target for the <30s metric: measure at device test
- Q2 desktop save destination: implemented the proposed rfd dialog; veto possible at device test
- Q3 double-confirm (type RESTORE) for other-lineage import: NOT implemented (single confirm with reinforced red warning); add on request
- Q4/Q5/Q6: backlog per RFC
