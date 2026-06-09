---
artifact: "docs/rfcs/0004-multidevice-sync/RFC.md"
artifact_kind: "rfc"
engine_tier: "solo"
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: "shipped"
pass_scope: "T01-T05 (SQLite sync foundation) + T06/T08 (spikes Noise + appairage)"
updated: "2026-06-09"
---

# Trace Ledger: Synchronisation multi-appareils (LAN, sans serveur)

> Single source of truth for progress. A fresh session reads ONLY this file to resume. One row per task/T-id.

## DAG (topological, from RFC section 10)

- Roots (no deps): T01, T06 (spike), T08 (spike)
- SQL-core chain: T01 -> T02 -> T03 -> {T04, T05}
- Vectors chain: T01 -> T09 -> T10 -> {T11, T12 -> T13}
- Transport chain: T06 -> T14 -> {T15 (also needs T08), T17 (also needs T05)}
- At-rest: T01 -> T16
- Sync: T17 -> {T18 -> T19 (also T05), T20, T23}; {T18, T19, T23} -> T24

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Notes |
|------|---------------|--------|---------------|--------|-------|
| T01 | C1 | done | `db/schema.rs` | solo | Migration V10 additive: sync_row_meta, sync_seq, sync_peers, sync_conflicts, chunks. V1-V9 intactes. |
| T02 | C2 | done | `db/mod.rs` | solo | device_id (settings) + recursive_triggers=ON + apply marker connection-local via fonction SQL `sync_is_applying()` (feature rusqlite `functions`). |
| T03 | C3 | done | `db/sync_meta.rs`, `db/schema.rs` | solo | Triggers AFTER INSERT/UPDATE générés; bump version_vector (JSON) + origin_seq atomique; guard `sync_is_applying()=0 AND device_id NOT NULL`. |
| T04 | C4 | done | (triggers couvrent) + tests | solo | set_audio_transcription/add_note_to_folder trackés par les triggers globaux (aucune réécriture des chemins create/update). Connexion fraîche testée. |
| T05 | C5 | done | `db/note_repo.rs`, `folder_repo.rs`, `attachment_repo.rs`, `conversation_repo.rs`, `note_reminder_repo.rs`, `sync_meta.rs` | solo | tombstone applicatif (note + enfants) avant DELETE, en transaction; CASCADE conservé; state='tombstone' -> deleted=1. |
| T06 | C6 | done | `services/sync/mod.rs`, `services/sync/transport.rs` | solo | SPIKE résolu. snow 0.10 default-resolver (pur Rust: chacha20poly1305 + curve25519-dalek, AUCUN ring/aws-lc/cmake). Cross-compile aarch64-apple-ios prouvé (make all -> installé device). Handshake XXpsk3 in-memory testé: roundtrip OK + PSK mismatch rejeté. Handshake sur socket TCP réel = T14. |
| T08 | C7 | done | `services/sync/peers.rs` | solo | SPIKE résolu. PairingPayload (device_id, addr, port, psk 32o, static_pubkey) encode/decode `flowflow://pair#<b64url(json)>`; PSK via getrandom; QR SVG via qrcode 0.14 (cross-compile iOS OK); repli IP `parse_manual_addr`. mDNS: décision déjà documentée RFC §6 (QR/IP primaire, Bonjour-système v2, mdns-sd exclu device). Scan caméra + connexion réelle = T15. |
| T09 | C8 | todo | `services/embed.rs` | - | Id chunk note déterministe. |
| T10 | C9 | todo | `services/embed.rs`, `services/vectordb.rs` | - | BLOB f32 + scope par préfixe (store_chunks CRITICAL). |
| T11 | C10 | todo | `services/embed.rs` | - | Backfill atomique. |
| T12 | C11 | todo | `services/sync/reconcile.rs`, `services/vectordb.rs` | - | reconstruct_from_blob. |
| T13 | C12 | todo | `services/sync/reconcile.rs`, `ui/mod.rs` | - | Boot reconcile pass. |
| T14 | C13 | todo | `services/sync/transport.rs` | - | Noise handshake + AEAD. |
| T15 | C14 | todo | `services/sync/peers.rs`, `ui/` | - | Appairage. |
| T16 | C15 | todo | `platform/ios/sync_ffi.rs`, `db/mod.rs` | - | NSFileProtection. |
| T17 | C16 | todo | `services/sync/protocol.rs` | - | HELLO/PUSH/ACK resumable. Utilisera Database::set_applying (déjà en place). |
| T18 | C17 | todo | `services/sync/conflict.rs` | - | Merge VV + sync_conflicts. |
| T19 | C18 | todo | `services/sync/peers.rs`, `reconcile.rs` | - | Tombstone GC + add-wins-resurrect (utilisera mark_entity_updated). |
| T20 | C19 | todo | `ui/`, `services/sync/mod.rs`, `platform/ios/sync_ffi.rs` | - | Déclencheurs sync. |
| T23 | C20 | todo | `services/sync/protocol.rs`, `db/note_reminder_repo.rs` | - | Exclusions + merge rappels. |
| T24 | C21 | todo | tests + manuel | - | E2E iPhone+Mac. |

T07, T21, T22: DESCOPED v1 (audio files not synced).

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-02 | plan | locked | DAG + contract.md; 21 active tasks. |
| step-03 | engine | solo | Critical/medium + DB migration + projet "une étape à la fois" -> série solo. |
| step-04 | risk-boundary (DB migration) | proceeded | Mirko a choisi "Fondation SQLite (T01-T05)" qui inclut la migration V10 additive. Code seulement; la vraie base n'est touchée que quand Mirko build. |
| step-05 | adversarial review | 1 BLOCKER + 0 MAJOR + 4 MINOR | 4 agents (SQL/zéro-perte/rust-compile/scope). rust-compile clean. |
| step-05 | BLOCKER fix | done + verified | Trigger ne peut pas lire sqlite_temp_master -> marqueur connection-local via fonction SQL `sync_is_applying()`. Repro SQLite autonome: PASS. |
| step-05 | MINOR fix | done | delete_folder double-bump sous-dossiers retiré (le trigger AU couvre via recursive_triggers). |
| passe 2 | spikes T06/T08 | done | snow + qrcode cross-compilent iOS (make all installe sur device); 8 tests host verts (3 transport + 5 appairage); fmt + clippy 0. Pas de fan-out: spikes petits, isolés (nouveau module, 0 modif de symbole existant hors `pub mod sync;`). |

## HALT events

- none

## Pass log

- 2026-06-09: passe 1 (T01-T05) shipped + commité/poussé sur development. 7 fichiers src modifiés + 1 nouveau (sync_meta.rs) + Cargo.toml (feature functions) + tests/sync_meta_test.rs (12 tests).
- 2026-06-09: passe 2 (spikes T06/T08) shipped. Nouveau module `src/services/sync/{mod,transport,peers}.rs` + `pub mod sync;` dans services/mod.rs + Cargo.toml (snow, qrcode, base64, getrandom) + tests/sync_transport_test.rs (3) + tests/sync_pairing_test.rs (5). make all = build iOS + install device OK. En attente: validation Mirko, puis passe 3 = chaîne vecteurs (T09->T13, indépendante du transport) OU transport réel (T14->T15).
- MINOR connus laissés tels quels (intentionnels, non bloquants): update_note multi-statement = N bumps (CRDT-correct); note_reminders state='active' via UPDATE un-tombstone la méta (state fait autorité, RFC MAJOR 16).
