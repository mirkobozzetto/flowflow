---
artifact: "docs/rfcs/0004-multidevice-sync/RFC.md"
artifact_kind: "rfc"
locked: "2026-06-09"
---

# Definition of Done: Synchronisation multi-appareils (LAN, sans serveur)

> Immutable target. Every item below is a concrete, checkable condition the final verification bundle validates against. Requirement changes get a NEW entry; never silently rewrite an existing line.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | Migration V10 sur COPIE de la vraie base v1.0; 0 perte; app démarre; idempotente | RFC T01 | `cargo test` migration + app boot (device) |
| C2 | `device_id` stable global; `recursive_triggers` actif; marqueur apply connection-local pour la connexion sync | RFC T02 | unit test + Read-back db/mod.rs |
| C3 | Un update bumpe l'entrée device + `origin_seq` unique; connexion sync (apply) = no-op; pas de collision concurrente | RFC T03 | unit test concurrent |
| C4 | Edit via CHAQUE chemin (y.c. threads embed) incrémente le version_vector (pas juste "row méta existe") | RFC T04 | unit test per-path |
| C5 | Supprimer note avec attachment+audio+rappel -> tombstone sur TOUS; tests cascade verts; `state='tombstone'` -> méta deleted=1 | RFC T05 | unit test tombstone + cascade tests |
| C6 | Spike Noise `snow` XXpsk3 build aarch64-apple-ios+sim; handshake OK sur appareil | RFC T06 | device build + handshake (USER) |
| C7 | Spike appairage QR/IP: scan/saisie -> connexion; repli IP documenté | RFC T08 | device test (USER) |
| C8 | Nouveaux chunks note ont un id déterministe `note:{id}:{idx}`; attachment inchangé | RFC T09 | unit test + Grep embed.rs |
| C9 | Note multi-chunks -> N BLOB; note + attachment coexistent; édition -> 0 orphelin; tests RAG verts | RFC T10 | RAG integration tests |
| C10 | Backfill: flag `settings` une-fois; 0 row à id aléatoire restant; notes existantes ont leur BLOB | RFC T11 | integration test backfill |
| C11 | Supprimer `vectordb/` -> reconstruit depuis BLOB; 0 appel embedding; RAG remarche sans re-consentement | RFC T12 | recovery integration test |
| C12 | Convergence idempotente boot SQLite<->LanceDB; pas de course avec un 2e thread LanceDB | RFC T13 | integration test reconcile |
| C13 | Canal Noise chiffré+authentifié; empreinte/PSK invalide -> refus | RFC T14 | device test (USER) |
| C14 | Appairer iPhone+Mac; clé/empreinte invalide refusée | RFC T15 | device test (USER) |
| C15 | NSFileProtection `CompleteUntilFirstUserAuthentication` sur `.db`/`-wal`/`-shm`; lock-mid-sync -> 0 IOERR/0 corruption | RFC T16 | device test (USER) |
| C16 | Push `origin_device=moi AND origin_seq > watermark`; idempotent; coupure mid-PUSH -> reprise cohérente | RFC T17 | integration + device test |
| C17 | Édition des 2 côtés (horloges +10s) -> 1 courant (enfants intacts) + 1 entrée sync_conflicts; 0 écrasement | RFC T18 | unit/integration merge test |
| C18 | Suppression note+enfants: 0 résurrection après 3 syncs; ajout enfant concurrent ressuscite parent; restauré -> full-state | RFC T19 | integration test |
| C19 | Sync démarre dès détection du pair; indicateur visible; fenêtre de grâce au background | RFC T20 | device test (USER) |
| C20 | Clés API ne traversent pas; `pending_transcriptions` ignoré; rappel synced sans collision UNIQUE ni double notif | RFC T23 | unit/integration test |
| C21 | E2E iPhone+Mac: 0 perte, 0 doublon, 0 embedding 2e appareil, transcription cross-device, <60s/500 notes, 0 octet tiers | RFC T24 | device E2E (USER) |

## Out of scope (never build)

- PAS de migration libSQL (on reste sur rusqlite).
- PAS de serveur/cloud/Turso/VPS ni la couche réseau du PRD `lan-serve` (relais VPS).
- PAS de sync "depuis n'importe où" sans pair joignable (LAN uniquement).
- PAS de collaboration temps réel multi-utilisateurs; PAS de CRDT / merge ligne-à-ligne tiers.
- PAS de sync distante (Tailscale); PAS d'Android.
- AUDIO FICHIERS descopés v1 (T07/T21/T22): seule la transcription voyage. Aucun codec, aucun transfert binaire d'audio.

## Edit scope

- `src/db/schema.rs`, `src/db/mod.rs`
- `src/db/note_repo.rs`, `folder_repo.rs`, `attachment_repo.rs`, `conversation_repo.rs`, `note_reminder_repo.rs`
- `src/services/embed.rs`, `src/services/vectordb.rs`
- `src/services/sync/{mod,meta,conflict,reconcile,protocol,transport,peers}.rs` (new)
- `src/platform/ios/sync_ffi.rs` (new sub-module)
- `src/ui/` (pairing screen, sync button/indicator, conflicts view)
- `tests/` (migration, tombstone, merge, RAG regression, reconcile)
