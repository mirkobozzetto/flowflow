# Fondation sync SQLite (T01-T05) - ce qui a été fait et comment tester

Passe 1 du RFC 0004 (multi-device sync). 100% rusqlite, additif, zéro réseau, zéro
FFI iOS. But: poser la couche de suivi des changements qui rendra la sync possible,
sans rien casser des données v1.0. Rien n'est encore synchronisé entre appareils.

## 1. Ce que ça fait, en clair

Chaque écriture locale (créer/éditer/supprimer une note, un dossier, un lien, etc.)
laisse une trace dans une table technique `sync_row_meta`. Cette trace dit:
- quelle entité a changé (`entity_kind`, `entity_id`),
- de combien de fois CET appareil l'a modifiée (`version_vector`, un compteur par appareil),
- un numéro de séquence local croissant (`origin_seq`) pour pousser "ce qui a changé depuis",
- si elle est supprimée (`deleted=1`, un "tombstone").

Plus tard (passes suivantes), la sync poussera ces traces au pair et fusionnera sans
jamais rien perdre. Pour l'instant, on ne fait que PRODUIRE ces traces, correctement.

Pourquoi c'est important: une suppression non tracée "ressuscite" chez le pair, et une
fusion à l'horloge murale perd des données en cas de décalage d'horloge. La fondation
évite les deux: tombstones explicites + compteur de version (jamais l'horloge).

## 2. Le mécanisme (3 idées)

1. Triggers (déclencheurs SQL) `AFTER INSERT`/`AFTER UPDATE` sur les 8 tables synchronisées.
   N'importe quelle connexion qui écrit -> la trace est posée automatiquement. C'est
   nécessaire car l'app ouvre beaucoup de connexions (UI, threads d'embedding...): pas
   de "setup par connexion" à oublier. Le `origin_seq` est alloué de façon atomique
   (`UPDATE sync_seq +1` puis relecture; WAL = un seul writer, donc pas de collision).

2. Tombstones à la suppression. Les triggers `AFTER DELETE` sur cascade ont une
   sémantique SQLite piégeuse, donc les suppressions sont tracées côté Rust: chaque
   chemin de delete écrit le tombstone de la ligne ET de ses enfants cascadés AVANT le
   `DELETE` physique, le tout dans une transaction. Le `ON DELETE CASCADE` fait toujours
   le ménage physique.

3. Marqueur "apply" connection-local. Quand la sync (future) applique un lot venu du
   pair, ses écritures ne doivent PAS être re-tracées comme locales. Une fonction SQL
   `sync_is_applying()` (propre à chaque connexion) le signale: les triggers deviennent
   no-op sur la seule connexion de la sync, pendant que les autres connexions continuent
   de tracker normalement (donc aucune écriture locale concurrente n'est perdue).

## 3. Fichiers touchés

| Fichier | Quoi |
|---------|------|
| `src/db/schema.rs` | + Migration V10: 5 tables (`sync_row_meta`, `sync_seq`, `sync_peers`, `sync_conflicts`, `chunks`). V1-V9 inchangées. |
| `src/db/sync_meta.rs` | NOUVEAU. Génère/installe les triggers, gère `device_id`, seed des lignes v1.0, helper `tombstone_entity`. |
| `src/db/mod.rs` | `PRAGMA recursive_triggers=ON`; fonction `sync_is_applying()` + flag connection-local; `set_applying()`; hook V10 + `init_sync()`. |
| `src/db/note_repo.rs` | `delete_note` (note+enfants), `delete_audio`, `delete_all_audios` tombstonés. |
| `src/db/folder_repo.rs` | `delete_folder`, `remove_note_from_folder` tombstonés. |
| `src/db/attachment_repo.rs` | `delete_attachment`, `delete_attachments_for_note` tombstonés. |
| `src/db/conversation_repo.rs` | `delete_conversation` (conv + messages) tombstoné. |
| `src/db/note_reminder_repo.rs` | `delete_note_reminder`, `delete_reminders_for_note` tombstonés. |
| `Cargo.toml` | rusqlite feature `functions` (pour `sync_is_applying()`). |
| `tests/sync_meta_test.rs` | NOUVEAU. 12 tests (migration, tracking, tombstones, apply, reparent). |

Pas touché: les chemins `create_*`/`update_*` (les triggers s'en chargent), donc zéro
risque sur les symboles à fort blast radius (`store_chunks` CRITICAL n'est pas dans cette passe).

## 4. Revue qualité (déjà faite)

4 agents adversariaux ont relu le code (zéro-perte, SQL, compilation Rust, scope).
Résultat: 1 BLOCKER, 0 MAJOR, compilation clean.

- BLOCKER (corrigé): un trigger SQLite ne peut pas lire `sqlite_temp_master`; le premier
  mécanisme (marqueur = table temp) aurait tué toute écriture après V10. Remplacé par la
  fonction `sync_is_applying()`. Vérifié empiriquement (repro SQLite autonome: PASS).
- MINOR laissés (intentionnels): un edit multi-champs compte N bumps de version
  (correct en CRDT); réactiver un rappel (`state='active'`) lève son tombstone (le `state`
  fait autorité pour les rappels, décision RFC).

## 5. Comment tester en local

### A. Tests automatiques (host, rapide) - À LANCER PAR TOI

```
cargo test --test sync_meta_test
```
Attendu: `test result: ok. 12 passed`. Ça couvre: migration V10, tracking insert/update,
unicité de `origin_seq`, tracking via connexion fraîche, tombstones note+enfants,
`state=tombstone -> deleted=1`, marqueur apply, reparent de sous-dossier.

Régressions (les suppressions CASCADE doivent rester vertes):
```
cargo test --test attachment_test
cargo test --test note_reminder_test
cargo test
```
Si `cargo test` râle sur la feature `mobile` par défaut sur Mac, utilise ta commande de
test habituelle du projet.

### B. Sur simulateur (voir le tracking vivant)

```
make dev
```
1. Crée 2-3 notes, mets-en une dans un dossier, supprime une note.
2. Inspecte la base (le chemin est imprimé au boot par `[db] opening ...`):
```
sqlite3 <chemin>/flowflow.db "SELECT entity_kind, entity_id, deleted, origin_seq FROM sync_row_meta ORDER BY origin_seq;"
```
   - Chaque note/dossier/lien a une ligne.
   - Une note éditée a un `origin_seq` qui a monté.
   - Une note supprimée + ses enfants ont `deleted=1`.
```
sqlite3 <chemin>/flowflow.db "SELECT * FROM _migrations;"          # contient version 10
sqlite3 <chemin>/flowflow.db "SELECT value FROM settings WHERE key='sync_device_id';"  # un UUID stable
```

### C. Sur ton iPhone (le vrai test non destructif) - À FAIRE PAR TOI

IMPORTANT: fais un backup d'abord (RFC 0001) - ça touche ta vraie base v1.0.
```
make ddev
```
- L'app doit démarrer normalement (la migration V10 s'applique au 1er lancement).
- Toutes tes notes/dossiers/rappels existants sont là (migration additive, 0 perte).
- Le seed remplit `sync_row_meta` pour tes données existantes (une fois).

## 6. Ce qui n'est PAS dans cette passe

Rien ne se synchronise encore. Pas de réseau, pas d'appairage, pas de vecteurs BLOB, pas
de merge. Prochaines passes: spikes T06 (`snow` iOS) + T08 (QR/IP), puis transport, puis
réconciliation. Cf. `trace.md` (DAG + statut) et `RFC.md` section 10.

## 7. Statut

Code écrit + revu + corrigé. Non commité (en attente de ton build/test). Si les tests
passent et l'app démarre sur device avec tes données intactes, on enchaîne la passe 2.
