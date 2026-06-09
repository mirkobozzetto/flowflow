---
artifact: "docs/rfcs/0004-multidevice-sync/RFC.md"
artifact_kind: "rfc"
engine_tier: "solo"
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: "shipped"
pass_scope: "T01-T05 (foundation) + T06/T08 (spikes) + T09-T13 (vecteurs) + T14/T15 (transport Noise TCP + appairage)"
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
| T09 | C8 | done | `services/embed.rs` | solo | Id chunk note `note:{id}:{idx}` (était Uuid aléatoire); attachment `att:{id}:{idx}` inchangé. |
| T10 | C9 | done | `services/embed.rs`, `services/vectordb.rs`, `db/chunk_repo.rs`, `db/mod.rs`, `db/note_repo.rs`, `db/attachment_repo.rs` | solo | `store_chunks`/delete scopés par PRÉFIXE d'id (corrige BLOCKER 5: signature/Arrow inchangées); BLOB f32 LE + content_hash sha256 dans table `chunks` SQLite (`replace_chunks` atomique); pre-delete nuisibles retirés. Fixes revue: purge chunks SQLite dans la tx de delete note/attachment + purge sur édition <50 chars. |
| T11 | C10 | done | `services/sync/reconcile.rs`, `services/vectordb.rs`, `db/chunk_repo.rs`, `services/embed.rs`, `ui/mod.rs` | solo | `backfill_legacy_chunks` (flag `chunks_backfilled_v10`): copie les vecteurs LanceDB legacy (note id aléatoire + att:) en BLOB SQLite déterministe, n'écrase jamais un owner déjà en SQLite, SKIP les attachments supprimés (fix revue). `migrate_chunk_dates` supprimé/fondu. reconcile dédoublonne ensuite (random ids -> orphelins supprimés). |
| T12 | C11 | done | `services/sync/reconcile.rs`, `services/vectordb.rs` | solo | `reconstruct_from_blob`: rebuild LanceDB depuis BLOB SQLite, parent attachment résolu par join, JAMAIS `ai.embed`, hors gate consent. `add_chunks`/`fetch_note_rows`/`all_ids`/`delete_ids` ajoutés (schéma Arrow inchangé). |
| T13 | C12 | done | `services/sync/reconcile.rs`, `ui/mod.rs` | solo | `reconcile_once` + `run_boot_reconcile` (1 thread, 1 runtime au boot). Diff ids SQLite(truth)/LanceDB: orphelins LanceDB supprimés + manquants reconstruits. Self-heal (fix revue): purge les chunks SQLite d'un owner supprimé -> convergence garantie. Garde dimension (skip BLOB corrompu). Idempotent. |
| T14 | C13 | done | `services/sync/transport.rs`, `tests/sync_transport_test.rs` | solo | Noise XXpsk3 sur TCP réel: framing length-prefixed (u16 BE), `SecureChannel` (AEAD, messages logiques chunkés > 65519o, header u32 longueur), `connect_secure`/`accept_secure` + timeouts 20s + nodelay. Vérif empreinte du static distant AVANT msg3 (fix revue: pas de fuite d'identité vers un mauvais pair). Garde anti-boucle frame vide (fix revue). 7 tests TCP localhost (roundtrip, gros msg 200ko, PSK invalide, empreinte invalide des 2 côtés). |
| T15 | C14 | done | `db/peer_repo.rs`, `services/sync/peers.rs`, `ui/sync_pairing.rs`, `ui/{mod,state,settings,top_bar}.rs`, i18n, `Dioxus.toml`, `tests/sync_pairing_test.rs` | solo | Appairage: `peer_repo` (CRUD `sync_peers` + `persist_pairing`/`delete_pairing` atomiques), identité statique persistée (settings), PSK par-pair (settings), `start_pairing_host` (listener + responder, fenêtre 300s, cancel), `join_pairing` (connect + vérif empreinte/device_id), `unpair`. UI `View::SyncPairing` (QR SVG + collage URI + liste pairs + dissociation), section Sync dans Settings, i18n FR/EN, `NSLocalNetworkUsageDescription`. Fixes revue: garde binding device_id (refus écrasement clé différente = anti-hijack), persistance atomique (peer+PSK 1 tx), accept loop survit aux erreurs transitoires, seam `FLOWFLOW_SYNC_ADVERTISE_ADDR` (tests hermétiques). 12 tests (URI, QR, identité, E2E host+join, PSK/empreinte/hijack refusés, unpair). |
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
| passe 3 | T09/T10 vecteurs | done | gitnexus_impact AVANT edit: store_chunks=CRITICAL (11 upstream/7 process) -> averti; signature/Arrow gardées intactes. Revue adversariale ultracode (4 lentilles): 2 BLOCKER (orphelins chunks SQLite sur delete note/attachment -> ressuscitables par T12) + 2 MAJOR (divergence, édition <50). BLOCKER corrigés (delete chunks atomique dans la tx), MAJOR <50 corrigé (purge count-guardé). MINOR/NIT différés T11/T13 (doublons legacy, content_hash write-only). 158 tests verts (single-thread), clippy 0, make all install device. |
| passe 5 | T14/T15 transport + appairage | done | gitnexus_impact AVANT edit: View enum (LOW, 1 caller widget) + SettingsView (LOW, 0 caller) -> ajouts sûrs. Revue adversariale ultracode (4 lentilles: crypto/correctness/persistence/ios-ui, 8 agents, 624k tokens): 3 MAJOR confirmés, 1 MAJOR réfuté (TOCTOU identité = fenêtre de quelques ms, non réaliste), 28 mineurs. 3 MAJOR + 5 robustesses corrigés AVANT commit: (a) hijack de binding device_id -> garde `bind_peer` (refus clé différente, refus id vide) + persistance atomique peer+PSK 1 tx; (b) QR non scannable -> hint reformulé (copier/coller, scan = futur scanner natif); (c) section Sync hors écran -> Settings `overflow-y-auto`; (d) fuite identité avant msg3 -> vérif empreinte avant envoi; (e) boucle frame vide -> garde no-progress; (f) accept loop survit ECONNABORTED/EINTR; (g) double-tap "Afficher code" -> garde reentry; (h) QR dead reset + max-width + seam test addr. 173 tests verts (single-thread), clippy 0, make all install device. |
| passe 4 | T11/T12/T13 vecteurs (fin chaîne) | done | gitnexus_impact AVANT edit: migrate_chunk_dates (LOW, 1 caller App) + vectordb_path (LOW, 0 caller) -> retrait/seam sûrs. Revue adversariale ultracode (4 lentilles, 16 agents): 12 findings, 7 "confirmés" par vérif. Tri: 2 ÉCARTÉS (BLOCKER+MAJOR "perte" = en fait nettoyage du cache LanceDB d'entités déjà supprimées par l'user = comportement voulu RFC §6, jumeau dismissed confirme). 3 RÉELS corrigés AVANT commit: (a) attachment supprimé -> backfill ré-injectait du junk SQLite non reconstructible (non-convergence) -> garde get_attachment + self-heal reconcile (purge owners morts); (b) tests fragiles multi-thread (env var globale) -> Mutex sérialisant; (c) BLOB corrompu (dim != 1536) -> panic chunks_to_batch -> garde dimension (skip + log). 163 tests verts (single-thread), clippy 0, make all install device. |

## HALT events

- none

## Pass log

- 2026-06-09: passe 1 (T01-T05) shipped + commité/poussé sur development. 7 fichiers src modifiés + 1 nouveau (sync_meta.rs) + Cargo.toml (feature functions) + tests/sync_meta_test.rs (12 tests).
- 2026-06-09: passe 2 (spikes T06/T08) shipped + commité/poussé (d3dcca5). Nouveau module `src/services/sync/{mod,transport,peers}.rs` + Cargo.toml (snow, qrcode, base64, getrandom) + tests/sync_transport_test.rs (3) + tests/sync_pairing_test.rs (5). make all = build iOS + install device OK.
- 2026-06-09: passe 3 (T09/T10 vecteurs) shipped. embed.rs (id déterministe + persist_chunk_blobs + purge <50), vectordb.rs (owner_prefix + store_chunks scopé + delete_note_own_chunks), db/chunk_repo.rs (nouveau: ChunkRecord, replace_chunks atomique, blob f32 LE, delete_chunks_for_owner), purge chunks dans delete_note/delete_attachment, Cargo.toml (sha2 0.10) + tests/chunk_blob_test.rs (6) + coexist dans rag_integration_test. Revue ultracode 4 lentilles -> 2 BLOCKER + 1 MAJOR corrigés avant commit. Reste passe 4: T11 backfill legacy (dédoublonne les random-id), T12 reconstruct_from_blob, T13 boucle reconcile boot.
- 2026-06-09: passe 4 (T11/T12/T13 vecteurs) shipped. reconcile.rs (nouveau: backfill_legacy_chunks, reconstruct_from_blob, reconcile_once, run_boot_reconcile), vectordb.rs (add_chunks/all_ids/delete_ids/fetch_note_rows + seam FLOWFLOW_VECTORDB_PATH, migrate_chunk_dates retiré), chunk_repo.rs (all_chunk_ids/distinct_chunk_owners/content_hash partagé), embed.rs (migrate_chunk_dates retiré, content_hash partagé), ui/mod.rs (run_boot_reconcile au boot) + tests/reconcile_test.rs (5). Revue ultracode 4 lentilles -> 3 findings réels corrigés avant commit. Chaîne vecteurs T09->T13 COMPLÈTE. Reste: chaîne transport (T14 Noise, T15 appairage), at-rest T16, sync T17-T20/T23, E2E T24.
- 2026-06-09: passe 5 (T14/T15 transport + appairage) shipped. transport.rs (SecureChannel AEAD + framing + connect/accept_secure + vérif empreinte pré-msg3 + garde frame vide), peers.rs (identité statique, PSK par-pair, bind_peer anti-hijack, start_pairing_host/join_pairing/unpair, accept loop résilient, seam advertise addr), db/peer_repo.rs (nouveau: sync_peers CRUD + persist_pairing/delete_pairing atomiques), ui/sync_pairing.rs (nouveau: écran QR + collage + liste pairs), ui/{state,mod,settings,top_bar}.rs (View::SyncPairing + section Sync + nav), i18n FR/EN (+18 clés), Dioxus.toml (NSLocalNetworkUsageDescription) + tests/sync_transport_test.rs (7) + tests/sync_pairing_test.rs (12). Revue ultracode 4 lentilles -> 3 MAJOR + 5 robustesses corrigés avant commit. Reste chaîne sync: T16 NSFileProtection, T17 protocole HELLO/PUSH/ACK (le vrai échange), T18 merge conflits, T19 tombstone GC, T20 déclencheurs/UI, T23 exclusions, T24 E2E iPhone+Mac.
- Device-only restant (Mirko): ouvrir l'app, vérifier notes/RAG/suppression OK après cette passe (le backfill+reconcile tourne au boot sur la vraie base; additif, idempotent, flag une-fois, zéro-perte par ordre SQLite-d'abord). Pour T14/T15: Settings -> Synchronisation -> "Afficher un code" sur un appareil, copier l'URI, "Connecter" sur l'autre (même Wi-Fi) -> les 2 doivent apparaître dans "Appareils appairés". NB: l'appairage écrit seulement la table sync_peers + l'identité/PSK; AUCUN échange de notes encore (c'est T17). Le scan QR caméra arrivera avec un scanner natif (URL scheme non enregistré pour l'instant; le collage d'URI est le chemin fonctionnel).
- Limites connues passe 5 (intentionnelles, notées par la revue, non bloquantes): host mono-connexion bloquant = un pair qui traîne (slowloris) peut occuper la fenêtre (mitigé par timeouts 20s/socket + fenêtre 300s); `local_lan_ip` peut annoncer l'IP cellulaire si le Wi-Fi n'a pas de route Internet (annonce une mauvaise adresse, pas de fuite); 1er "Connecter" peut échouer pendant le prompt iOS Local Network (réessayer); persistance host avant pair_ok = pair fantôme côté host si l'envoi échoue (récupérable par re-appairage/unpair). À traiter en durcissement T17+ si besoin.
- MINOR connus laissés tels quels (intentionnels, non bloquants): update_note multi-statement = N bumps (CRDT-correct); note_reminders state='active' via UPDATE un-tombstone la méta (state fait autorité, RFC MAJOR 16). Flake test cross-binaires (SQLite partagé `temp_dir/flowflow/flowflow.db` entre binaires lancés en parallèle par cargo, amplifié par triggers T05): intermittent, pré-existant, NON lié à cette passe; re-run vert. Mes tests reconcile sont isolés (open_at tempdir + seam vectordb + Mutex).
