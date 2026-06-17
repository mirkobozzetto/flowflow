---
artifact: "docs/rfcs/0001-data-backup-export/RFC.md"
artifact_kind: "rfc"
locked: "2026-06-11"
---

# Definition of Done: Backup, export & restore des données FlowFlow (RFC 0001 rev2)

> Immutable target. Every item below is a concrete, checkable condition the final verification bundle validates against. Requirement changes get a NEW entry; never silently rewrite an existing line.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | 4 scrub const lists (SENSITIVE_SETTINGS, SENSITIVE_SETTING_PREFIXES, DEVICE_LOCAL_SETTINGS, DEVICE_LOCAL_SETTING_PREFIXES) single-sourced in settings_repo.rs, tested; manifest serde round-trips; schema_version computed dynamically from MAX(MIGRATIONS); device_id included | RFC T1 | cargo test (manifest + scrub consts) |
| C2 | VACUUM INTO snapshot on a dedicated read-only connection (never the shared Mutex); scrub on raw connection with journal_mode=MEMORY + secure_delete=ON then VACUUM; no -wal/-shm/-journal sidecar next to staged file; capture-then-scan secret test green; export gated on chunks_backfilled_v10 with synchronous backfill fallback | RFC T2 | cargo test (snapshot, scrub, gate) |
| C3 | WAV collected via note_audios.file_path + resolve_audio_path (never dir-walk); missing WAV tolerated + recorded in manifest.audio_missing; CRC32 per zip entry; zip adds explicit entries only | RFC T3 | cargo test (collection, manifest entries) |
| C4 | iOS share via UIActivityViewController; desktop save via rfd + reveal in Finder; clean cancellation; stale staging sweep on next export | RFC T4 | device/desktop manual test (Mirko) |
| C5 | Import validation 100% read-only on raw connection (staged bytes unchanged, tested); refuse schema_version < 10 or > app; anti-tamper cross-checks (MAX(_migrations) == manifest, device_id == manifest, counts == manifest, embedded WAV vs audio_missing, no sidecar); staged DB + dirs fsynced; sync_restored_pending + sync_restored_floor written INTO staged DB at phase 1; pending_restore/ outside Documents; blocking "restart required" screen + SyncEngine + TranscriptionManager stopped + WillEnterForeground observer | RFC T5 | cargo test + device manual test |
| C6 | Phase 2 swap at top of main() before any db_path(): re-validation, WAV copy with CRC collision handling (old file moved to restore_bak/ first), checkpoint TRUNCATE + sidecar removal, single-file move old db to restore_bak/, vectordb/ purge (resolved path), fsync + single rename = commit + fsync; two-factor commit predicate; orphan-cleanup skipped first post-restore boot; restore_bak/ purged at NEXT successful boot; rollback mandatory else boot abort; fault-injection green at every state-table row | RFC T6 | cargo test (fault injection per state) |
| C7 | HELLO carries restored{floor} (protocol version bump); per-peer sync_restored_done_{peer} marks force full-state until all peers marked; authority exempts origin_seq > floor; HLC guard routes vv-dominated-but-newer to Concurrent; rebind requires explicit confirm on old-binding holder, preserves sync_peers row, clears ack book | RFC T7 | cargo test (3-device sync scenarios) |
| C8 | Settings UI: export/import buttons, confirm with reinforced other-lineage warning (manifest.device_id), progress states, forced re-consent, re-pair invite, index-rebuild banner, i18n FR/EN | RFC T8 | device/desktop manual test (Mirko) |
| C9 | Test suite: round-trip on PRE-POPULATED env (divergent pre-existing vectordb); capture-then-scan; fault injection per state; 3 devices no resurrection; post-restore creations preserved; fresh edit not overwritten (HLC guard); WAV collision; virgin device; archive < V10 refused | RFC T9 | cargo test full suite |

## Out of scope (never build)

- No auto sync, no managed cloud backup
- No scheduled background backup
- No merge on import (post-restore reconvergence goes through sync)
- No selective export
- No password encryption (secrets excluded, not included-encrypted)
- No interoperable format
- No OS notification re-scheduling for restored reminders (backlog)

## Edit scope

- `src/services/backup.rs` (NEW)
- `src/platform/ios/share.rs` (NEW)
- `src/db/settings_repo.rs`, `src/db/mod.rs`, `src/db/note_repo.rs`
- `src/main.rs`
- `src/services/sync/protocol/session.rs`, `src/services/sync/protocol/apply.rs`
- `src/services/sync/peers.rs`, `src/services/sync/engine.rs`, `src/services/sync/reconcile.rs`
- `src/platform/ios/sync_ffi.rs`, `src/platform/ios/mod.rs`, `src/platform/ios/picker.rs`
- `src/ui/mod.rs`, `src/ui/settings.rs`
- `Cargo.toml` (rfd, desktop only)
- `tests/`
