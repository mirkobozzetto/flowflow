---
type: tasks
slug: long-local-transcription
source_brief: docs/brief/long-local-transcription/brief.md
---

# Tasks: Transcription locale des longs enregistrements

## Relevant Files

- `src/infrastructure/transcription/whisper.rs` - `run_whisper` traite tout
  l'audio d'un bloc ; `load_wav_mono_16k` charge le fichier entier.
- `src/application/transcription_manager/processing.rs` - `process_local`
  lance Whisper et publie `Polling { elapsed_s }`.
- `src/application/transcription_manager/mod.rs` - `resume_pending`
  relance les transcriptions au démarrage, aujourd'hui depuis zéro.
- `src/application/transcription_manager/job.rs` - `JobStatus`, où la
  progression doit vivre.
- `src/infrastructure/persistence/pending_transcription_repo.rs` - ligne
  de reprise, sans point d'avancement aujourd'hui.
- `src/infrastructure/persistence/schema.rs` - migration additive si le
  point de reprise demande une colonne.
- `src/infrastructure/platform/ios/sync_ffi.rs` - observateurs
  arrière-plan / premier plan existants, motif à réutiliser.
- `src/ui/notes/audio_section.rs` - libellé « Transcription en cours ·
  mm:ss ».

## Tasks

Ordered. Each task closes the acceptance criteria it names.

## T01 - Pause propre en arrière-plan, reprise au retour [#192](https://github.com/mirkobozzetto/flowflow/issues/192)

Closes: AC1, AC2

- [x] Passer en arrière-plan pendant une transcription locale arrête
      Whisper avant tout nouveau travail GPU, sans crash.
- [x] Revenir au premier plan relance la transcription en attente.
- [x] Verrouiller l'iPhone en pleine transcription ne produit aucun
      rapport de crash `ggml_metal_synchronize`.

## T02 - Reprise au point atteint [#193](https://github.com/mirkobozzetto/flowflow/issues/193)

Closes: AC3, AC4

- [x] L'avancement est enregistré au fil de la transcription et survit à
      un arrêt forcé de l'app.
- [x] Une reprise ne retraite pas l'audio déjà transcrit.
- [x] Les mots transcrits avant et après une reprise gardent leur position
      exacte dans l'enregistrement (toucher un mot saute au bon moment).

## T03 - Pourcentage d'avancement visible [#194](https://github.com/mirkobozzetto/flowflow/issues/194)

Closes: AC5

- [x] La note en cours de transcription locale affiche un pourcentage qui
      progresse, y compris après une reprise.

## T04 - Écran allumé pendant la transcription [#195](https://github.com/mirkobozzetto/flowflow/issues/195)

Closes: AC6

- [x] L'écran ne se verrouille pas tout seul tant qu'une transcription
      locale tourne au premier plan.
- [x] Le verrouillage automatique revient dès la fin, l'échec ou l'abandon
      de la transcription.

## T05 - La note de 66 minutes aboutit sur l'iPhone [#196](https://github.com/mirkobozzetto/flowflow/issues/196)

Closes: AC7

- [x] Installer sur l'iPhone, ouvrir la note du 28/09, la laisser reprendre,
      verrouiller puis déverrouiller une fois en cours de route.
- [x] La note obtient son texte complet et sa lecture synchronisée.
