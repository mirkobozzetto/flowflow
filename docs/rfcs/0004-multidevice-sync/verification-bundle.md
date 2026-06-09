---
artifact: "docs/rfcs/0004-multidevice-sync/RFC.md"
stack: "rust / cargo"
generated: "2026-06-09"
ran_by: "user"
pass_scope: "T01-T05 (SQLite sync foundation)"
---

# Verification Bundle: Sync foundation (T01-T05)

> ship ne lance PAS ces commandes (ta règle: tests/builds = toi). Lance-les
> toi-même. Chaque ligne dit ce qu'elle prouve. Stack détectée: rust/cargo.

## Safe checks (tu les lances)

| Command | Validates | Expected pass signal |
|---------|-----------|----------------------|
| `cargo test --test sync_meta_test` | C1-C5 (les 12 tests de la fondation) | `test result: ok. 12 passed` |
| `cargo test --test attachment_test` | C5: régression CASCADE attachments verte | tous verts, dont `test_cascade_delete_note_removes_attachments` |
| `cargo test --test note_reminder_test` | C5: régression CASCADE reminders + tombstone state | tous verts |
| `cargo test` | régression globale (aucune table/flow cassé) | tout vert |
| `cargo build` (ou `make build`) | compile, feature mobile | exit 0 |

Si `cargo test` se plaint de la feature `mobile` par défaut sur Mac, utilise ta
commande de test habituelle du projet (les tests existants tournent déjà avec).

## Device / réel (USER ONLY: ship ne lance jamais)

| Command | Validates | Warning |
|---------|-----------|---------|
| `make dev` puis créer/éditer/supprimer une note sur le simulateur | migration V10 sur une base, app démarre, tracking vivant | la migration s'applique au 1er lancement |
| `make ddev` sur ton iPhone qui a la vraie base v1.0 | C1: migration non destructive sur tes vraies données | BACKUP D'ABORD (RFC 0001). mutates la vraie DB |

Inspection manuelle de la base (sur le simulateur, après quelques notes):
- `sqlite3 <flowflow.db> "SELECT entity_kind, entity_id, deleted, origin_seq, version_vector FROM sync_row_meta;"`
  -> chaque note/folder/etc. a une ligne; un edit a `origin_seq` qui monte; un delete a `deleted=1`.
- `sqlite3 <flowflow.db> "SELECT * FROM _migrations;"` -> contient la version 10.
- `sqlite3 <flowflow.db> "SELECT value FROM settings WHERE key='sync_device_id';"` -> un UUID stable.

## Contract coverage (cette passe)

- C1 (migration V10 additive, 0 perte, idempotente) -> `test_migration_v10_creates_sync_tables`, `test_v10_reopen_is_idempotent_and_nondestructive` + device test sur vraie base.
- C2 (device_id stable, recursive_triggers, apply marker) -> `test_apply_marker_makes_triggers_noop` + Read-back `db/mod.rs`.
- C3 (insert/update bump version + origin_seq unique, apply=no-op) -> `test_insert_note_tracks_local_version`, `test_update_note_bumps_version_and_seq`, `test_origin_seq_is_unique_and_monotonic`, `test_apply_marker_makes_triggers_noop`.
- C4 (chaque chemin tracke, y.c. connexion fraîche/embed) -> `test_fresh_connection_still_tracks`, `test_set_audio_transcription_is_tracked`, `test_notes_folders_link_tracked_and_tombstoned`.
- C5 (tombstone note+enfants; cascade verte; state->deleted=1) -> `test_delete_note_tombstones_note_and_all_children`, `test_reminder_state_tombstone_maps_to_deleted`, + les 2 tests cascade existants.

- Hors de cette passe (C6-C21): transport Noise, appairage, vecteurs BLOB, merge,
  GC, E2E. À venir dans les passes suivantes (T06+).

## À valider en priorité (inconnues SQL, le test les couvre mais confirme)

1. Le JSON path à clé entre guillemets sur SQLite bundled (`json_set('{}', '$."uuid"', 1)`) - couvert par les tests, vérifié empiriquement en revue.
2. Le marqueur apply via fonction SQL `sync_is_applying()` (connection-local) - `test_apply_marker_makes_triggers_noop`.
3. Le SET NULL FK qui déclenche le trigger AU sous `recursive_triggers=ON` - `test_subfolder_reparent_on_parent_delete_is_tracked`.
4. La migration V10 sur une COPIE de ta vraie base v1.0 (0 perte, app démarre) - device, toi.
