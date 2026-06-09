# Passe 6 - At-rest + Protocole de sync (T16/T17)

RFC 0004. Le moteur de synchronisation: deux appareils appairés échangent leurs
lignes. Pas encore déclenché par l'UI (T20).

## T16 - Chiffrement au repos + checkpoint WAL

`src/platform/ios/sync_ffi.rs` (nouveau), `db/mod.rs`, `ui/mod.rs`.

- NSFileProtection classe `CompleteUntilFirstUserAuthentication` (PAS `Complete`,
  qui rendrait `-wal`/`-shm` illisibles écran verrouillé et corromprait un sync
  foreground) appliquée à `flowflow.db` + `-wal` + `-shm` à chaque open.
- `checkpoint_wal()` (`PRAGMA wal_checkpoint(TRUNCATE)`, flag busy surfacé en
  erreur) déclenché par un observer `UIApplicationDidEnterBackgroundNotification`
  (même pattern block2 que l'observer d'interruption audio).
- `PRAGMA busy_timeout=5000` sur toute connexion (les threads embed attendent au
  lieu d'échouer en SQLITE_BUSY pendant un apply).

## T17 - Protocole HELLO/PUSH/ACK

`src/services/sync/protocol/` (nouveau, 6 modules SRP):

| Module | Responsabilité |
|--------|----------------|
| `wire.rs` | Messages (Hello/Push/Ack), hint de routage, send/recv chiffrés |
| `catalog.rs` | Registre des entités synchronisées (kind -> table/colonnes/clé) |
| `vv.rs` | Version vectors: parse (corruption détectée), compare, join |
| `collect.rs` | Émetteur: énumération par watermark + payload + chunks (1 tx) |
| `apply.rs` | Récepteur: merge, archive de conflits, batch atomique |
| `session.rs` | Sessions bidirectionnelles + gardes HELLO |

- Push: `WHERE origin_device = moi AND origin_seq > watermark du pair`, batches
  croissants. Apply: rows + avance du watermark en UNE tx `BEGIN IMMEDIATE` ->
  une coupure (suspension iOS) laisse applied-and-acked ou rien; la reprise
  repart exactement du dernier batch acquitté.
- Merge VV: dominance -> apply verbatim / skip idempotent. Concurrence ->
  gagnant déterministe `(updated_hlc, origin_device)` (identique des 2 côtés),
  perdant ARCHIVÉ dans `sync_conflicts` (zéro perte), row ré-authorée
  localement (vv = join, seq frais) -> le join revient à l'autre appareil dans
  la MÊME session, méta convergente, pas de conflit fantôme répété.
- Vecteurs: les chunks BLOB voyagent DANS le payload de leur owner
  (note/attachment); l'embed bumpe la méta de l'owner après écriture des BLOBs
  (tx IMMEDIATE + garde anti-résurrection si l'owner a été supprimé entre-temps).
  Zéro re-embedding sur le 2e appareil.
- `set_applying` + `PRAGMA foreign_keys=OFF` pendant l'apply (l'ordre des seq ne
  respecte pas les FK; chaque enfant porte sa propre row/tombstone), garde RAII
  (rollback + applying=false + FK=ON sur tout chemin de sortie).

## Revue adversariale (ultracode, 4 lentilles, 29 agents)

21 findings confirmés (~14 racines), 4 réfutés. Corrigés avant commit:

- BLOCKER: watermark avançait au-delà de rows skippées sans trace -> toute
  décision de skip écrit la méta (durable); kind inconnu = échec de batch
  visible; `protocol_version` dans HELLO (skew de version refusé proprement).
- BLOCKER: seq dupliqué (bump embed hors tx) + TOCTOU résurrection -> tx
  `BEGIN IMMEDIATE` autour du check-alive + bump.
- MAJOR: origin forgé (row prétendant venir d'un autre appareil) refusé;
  restore-from-backup détecté au HELLO (erreur explicite, full-state = T19);
  apply DEFERRED abortable par un writer concurrent -> IMMEDIATE; tombstone
  rappel -> `state='tombstone'` tant que la note vit (state fait autorité).
- VV corrompu -> conflit forcé avec archive (jamais d'écrasement silencieux);
  garde dimension des chunks reçus; checkpoint busy surfacé.

Différés (notés, non bloquants): edit-vs-delete des enfants (T19), poison-pill
= stall visible (T20 surface l'erreur), merge rappels cross-id (T23),
`beginBackgroundTask` (T20).

## Tests (`tests/sync_protocol_test.rs`, 11, E2E sur TCP localhost, 2 DB réelles)

Bidirectionnel + idempotence (re-sync pousse 0), édition + méta convergente,
tombstone note+enfants+chunks, conflit concurrent (convergence + exactement 1
archive contenant le perdant), coupure mid-PUSH -> reprise sans perte ni
doublon, intent rappel dupliqué sans collision UNIQUE, tombstone rappel par
state, VV corrompu -> archive puis convergence, pair non appairé refusé
(localement ET côté host), folders/liens/conversations.

## Ce que ça NE fait PAS encore

Aucun déclencheur: l'app n'appelle ni `sync_with_peer` ni `serve_sync_once`.
T20 branche le bouton "Sync maintenant", le listener au premier plan, le sync
à la sauvegarde (debounced) et l'indicateur. T18 extrait/durcit le merge dans
`conflict.rs` + UI des conflits. T24 = E2E iPhone+Mac réels.
