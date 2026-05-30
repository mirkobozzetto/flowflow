---
feature: Import audio dans une note pour transcription
slug: audio-import-transcription
type: tasks
source_prd: docs/prd/audio-import-transcription/prd.md
stepsCompleted: [0, 1, 2, 3]
---

> ⚠️ Do NOT implement. This is the derived task list. Run `apex` (or the implementer) to execute.

## Relevant Files
- `src/platform/ios/picker.rs` - `open_file_picker(extensions)` existant (UIDocumentPicker, asCopy) à réutiliser pour les formats audio
- `src/ui/notes/menu.rs` - menu NoteDetail (point d'entrée "Importer un audio", à côté de l'import documents)
- `src/ui/notes/detail.rs` - NoteDetail (déclenchement import, états, progression, insertion du texte)
- `src/services/transcription/client.rs` - SonioxClient (transcribe format-agnostic ; timeout `MAX_POLLS`/`POLL_INTERVAL` ; `language_hints_strict` à assouplir pour l'auto-détection)
- `src/services/transcription/hesitations.rs` - nettoyage du transcript (réutilisé)
- `src/services/audio.rs` - dossier audio / `resolve_audio_path` (référence pour le staging temporaire du fichier importé)
- `src/db/note_repo.rs` - persistance du contenu de la note (`set_audio_transcription` / mise à jour du contenu)
- `src/services/embed.rs` - auto-embed du contenu transcrit (indexation recherche)
- `src/ui/consent.rs` + `get_setting("ai_consent")` / `soniox_api_key` - prérequis clé + consentement
- `src/services/i18n/locales/en.ftl` + `fr.ftl` - libellés UI (action, états, messages d'erreur)

## Tasks

- [ ] 1.0 Sélection audio + point d'entrée dans la note  _(PRD: stories 1, 2)_
  - [ ] 1.1 Ajouter l'action "Importer un audio" dans le menu de NoteDetail, distincte de l'import de documents.
  - [ ] 1.2 Ouvrir le sélecteur de fichiers restreint aux formats audio ciblés (m4a/AAC, mp3, wav, caf), un seul fichier.
  - [ ] 1.3 Refuser proprement un format non supporté (message clair, note intacte) avant tout traitement.
  - [ ] 1.4 Vérifier les prérequis tôt: clé Soniox présente + consentement IA donné, sinon message indiquant quoi configurer.

- [ ] 2.0 Pipeline import → transcription  _(PRD: stories 1, 5)_
  - [ ] 2.1 Récupérer le fichier choisi dans un emplacement temporaire (staging), hors espace exposé.
  - [ ] 2.2 Lancer la transcription du fichier via le service existant (format-agnostic).
  - [ ] 2.3 Activer l'auto-détection de langue pour ce flux (ne plus imposer le français strict à l'import).
  - [ ] 2.4 Nettoyer le fichier temporaire après transcription (réussie ou échouée): aucun audio conservé.

- [ ] 3.0 Support des fichiers longs (multi-heures) + progression  _(PRD: story 3)_
  - [ ] 3.1 Étendre la borne de durée de la transcription pour tenir des fichiers de plusieurs heures (timeout partagé import + live).
  - [ ] 3.2 Exposer une progression visible pendant le traitement (pas d'impression de blocage).
  - [ ] 3.3 Confirmer la borne haute cible (durée/taille) et l'appliquer comme garde explicite (cf. PRD Open questions).

- [ ] 4.0 Transcription en arrière-plan  _(PRD: story 4)_
  - [ ] 4.1 Exécuter la transcription en tâche de fond: l'app reste navigable pendant le traitement.
  - [ ] 4.2 Garantir que le résultat n'est pas perdu si l'utilisateur quitte la note ou navigue ailleurs.
  - [ ] 4.3 Signaler la fin de traitement quand l'utilisateur est ailleurs (signal discret ou visible au retour sur la note — cf. PRD Open questions).

- [ ] 5.0 Insertion du texte + indexation recherche  _(PRD: story 1)_
  - [ ] 5.1 Ajouter le texte transcrit au contenu de la note une fois la transcription prête.
  - [ ] 5.2 Déclencher l'auto-embed du contenu pour rendre l'import cherchable (recherche sémantique + chat).
  - [ ] 5.3 Vérifier que la note importée se comporte comme une note normale (recherche, tags, chat).

- [ ] 6.0 Gestion d'échec robuste  _(PRD: story 6)_
  - [ ] 6.1 Aucun texte partiel inséré en cas d'échec: la note reste strictement intacte.
  - [ ] 6.2 Messages d'erreur clairs et distincts par cause (délai dépassé, format refusé, clé absente, consentement manquant, réseau).
  - [ ] 6.3 Permettre de relancer l'import après échec sans re-créer la note.
  - [ ] 6.4 États visuels cohérents: en cours, succès, échec (avec raison) — libellés i18n FR/EN.

- [ ] 7.0 Tests & validation  _(PRD: acceptance criteria + success metrics)_
  - [ ] 7.1 Formats: importer et transcrire un échantillon de chaque format ciblé (m4a/AAC, mp3, wav, caf).
  - [ ] 7.2 Fichier long: transcrire un fichier ≥ 2 h jusqu'au bout sans échec de durée.
  - [ ] 7.3 Arrière-plan: lancer une transcription, naviguer ailleurs, vérifier le résultat présent au retour.
  - [ ] 7.4 Échec: format refusé / clé absente / consentement manquant → note intacte + message clair + retry.
  - [ ] 7.5 Indexation: retrouver le contenu importé via la recherche/le chat après transcription.
  - [ ] 7.6 Non-régression: l'enregistrement live continue de fonctionner après le relèvement du timeout partagé.
