---
feature: Backup, export & restore des données FlowFlow
slug: data-backup-export
type: tasks
source_prd: docs/prd/data-backup-export/prd.md
stepsCompleted: [0, 1, 2, 3]
---

> ⚠️ Do NOT implement. This is the derived task list. Run `apex` (or the implementer) to execute.

## Relevant Files
- `src/services/backup.rs` - nouveau module export/import (anticipé)
- `src/db/mod.rs` - chemin SQLite (`Documents/flowflow.db`), fermeture/réouverture DB
- `src/services/vectordb.rs` - chemin LanceDB (`Documents/vectordb`)
- `src/services/audio.rs` - dossier audio (`Documents/flowflow/recording_*.wav`)
- `src/db/settings_repo.rs` - clés API à exclure de l'export
- `src/platform/ios/mod.rs` - `documents_dir`, share sheet iOS, file picker
- `src/ui/settings.rs` - boutons Export / Import + confirmations
- `Cargo.toml` - crate `zip` (déjà présent via DOCX) pour l'archive

## Tasks

- [ ] 1.0 Format d'archive FlowFlow + manifest versionné  _(PRD: stories 1, 3, 5)_
  - [ ] 1.1 Définir la structure de l'archive (zip): un `manifest` (version de schéma, date, compteurs) + un dossier de données.
  - [ ] 1.2 Lister précisément ce qui entre: `flowflow.db`, `vectordb/`, fichiers audio WAV; acter l'exclusion explicite des clés API.
  - [ ] 1.3 Définir la règle de version: numéro de schéma de l'archive + politique de compatibilité (refus si incompatible).

- [ ] 2.0 Export complet vers archive, clés API exclues  _(PRD: stories 1, 4)_
  - [ ] 2.1 Rassembler les sources sous `Documents` (SQLite + `vectordb/` + audio) dans un staging cohérent.
  - [ ] 2.2 Produire une copie de la base SQLite sans les valeurs sensibles (clés `openai`/`anthropic`/`soniox`).
  - [ ] 2.3 Empaqueter staging + manifest en une archive unique.
  - [ ] 2.4 Écrire l'archive dans un emplacement partageable et retourner son chemin.

- [ ] 3.0 Partage de l'archive via share sheet natif iOS  _(PRD: story 2)_
  - [ ] 3.1 Exposer une fonction de partage iOS (UIActivityViewController) sur l'archive générée (AirDrop/Fichiers/mail/cloud).
  - [ ] 3.2 Gérer l'annulation et l'échec du partage proprement (pas d'état bloqué).

- [ ] 4.0 Import / restore replace total, atomic + validation  _(PRD: stories 3, 4, 5)_
  - [ ] 4.1 Sélectionner l'archive via le file picker iOS existant.
  - [ ] 4.2 Valider AVANT toute écriture: présence du manifest, version compatible, intégrité du contenu.
  - [ ] 4.3 Validation OK → remplacer atomiquement les données actuelles (SQLite + `vectordb/` + audio) par celles de l'archive.
  - [ ] 4.4 Validation KO → refuser avec message clair, données actuelles intactes (aucun état partiel).
  - [ ] 4.5 Après restore → rouvrir DB + vector store; ne PAS restaurer les clés API, inviter à les re-saisir.

- [ ] 5.0 UI Settings: Export / Import + confirmation du replace  _(PRD: stories 1, 2, 3)_
  - [ ] 5.1 Ajouter les boutons Export et Import dans `SettingsView`.
  - [ ] 5.2 Flux export: déclencher, montrer la progression, puis ouvrir le partage.
  - [ ] 5.3 Flux import: confirmation explicite "ceci va écraser vos données actuelles" avant le replace.
  - [ ] 5.4 États visuels: succès, échec (avec raison), invite à re-saisir les clés après import.

- [ ] 6.0 Tests & validation  _(PRD: acceptance criteria + success metrics)_
  - [ ] 6.1 Round-trip: export puis import sur base réaliste, 0 perte (notes/audio/tags/conversations + recherche sémantique OK).
  - [ ] 6.2 Exclusion clés: aucune clé API dans l'archive; clés non restaurées après import.
  - [ ] 6.3 Échec: archive corrompue + version incompatible → refus, données intactes.
  - [ ] 6.4 Appareil/app vierge: import restaure tout à l'identique.
  - [ ] 6.5 Mesurer la durée d'export sur le volume cible (à confirmer) et vérifier < 30 s.
