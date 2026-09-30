---
type: brief
slug: long-local-transcription
title: Transcription locale des longs enregistrements
status: shipped
created: 2026-09-30
next_action: Une transcription Whisper locale survit à l'arrière-plan, reprend là où elle s'est arrêtée et affiche sa progression.
resume_cmd: /ship docs/brief/long-local-transcription
base: dev
branch: feat/long-local-transcription
issues: "#192-#196"
---

# Transcription locale des longs enregistrements

## Problem

Un enregistrement d'une heure transcrit avec Whisper sur l'iPhone ne se
termine jamais. Dès que l'écran se verrouille ou que l'utilisateur change
d'app, FlowFlow plante. Au lancement suivant, la transcription repart de
zéro, puis replante. La note reste vide, avec « Transcription en cours »,
indéfiniment.

## Constat sur l'appareil (2026-09-30)

- Réglages : `stt_provider = whisper_local`, modèle `large-v3-turbo-q5_0`.
- Note `2d694da3…` créée le 28/09 à 11:16 : contenu vide, 0 mot.
- Enregistrement `recording_1790586967.wav` : 365,7 Mo, 3 994 s (66 min).
- Ligne `pending_transcriptions` toujours présente depuis le 28/09.
- Cinq rapports de crash FlowFlow (28/09 11:40, 30/09 22:54, 23:09,
  23:12…), tous identiques : `SIGABRT` dans `ggml_abort` appelé par
  `ggml_metal_synchronize`, pendant `whisper_full_with_state`.
- Durée de vie de l'app avant crash : entre 1 et 15 minutes, jamais assez
  pour finir 66 minutes d'audio.
- Aucun `JetsamEvent` ne vise FlowFlow : ce n'est pas la mémoire.

## Users

- L'utilisateur qui enregistre une réunion ou un entretien long et veut le
  texte sans envoyer l'audio dans le cloud.

## Goals

- Une transcription locale longue finit, même si l'utilisateur quitte l'app
  en cours de route.
- Le travail déjà fait n'est jamais perdu.
- L'utilisateur voit où en est la transcription.

## Acceptance criteria

- AC1 Verrouiller l'écran ou passer à une autre app pendant une
  transcription locale ne fait plus planter FlowFlow.
- AC2 Au retour dans l'app, la transcription se remet en marche toute seule,
  sans action de l'utilisateur.
- AC3 Après une pause, un crash ou un arrêt forcé de l'app, la transcription
  reprend au point atteint, jamais depuis le début.
- AC4 Le texte final d'une transcription interrompue puis reprise est
  continu, et la lecture synchronisée (toucher un mot saute au bon moment
  de l'audio) tombe juste sur toute la durée.
- AC5 Pendant une transcription locale, la note affiche un pourcentage
  d'avancement, pas seulement un chronomètre.
- AC6 Tant qu'une transcription locale tourne et que l'app est au premier
  plan, l'écran ne se verrouille pas tout seul ; il retrouve son
  comportement normal dès qu'elle se termine.
- AC7 Sur l'iPhone de Mirko, la note du 28/09 (66 min) obtient sa
  transcription complète.

## Success metrics

La note de 66 minutes bloquée depuis le 28/09 est transcrite. Plus aucun
rapport de crash `ggml_metal_synchronize` après un verrouillage d'écran
pendant une transcription.

## Out-of-scope

- Continuer la transcription écran verrouillé. Sur iPhone, iOS n'accorde
  pas le GPU en arrière-plan ; en CPU seul, 66 minutes de large-v3-turbo
  seraient beaucoup trop lentes. À rouvrir si Apple l'ouvre aux iPhone.
- La transcription Soniox (cloud) : son sondage reprend déjà après un
  redémarrage.
- L'enregistrement lui-même : il tient déjà une heure (mode audio
  d'arrière-plan).
- L'app Mac : pas de suspension ni d'interdiction GPU en arrière-plan.
- Une activité en direct sur l'écran verrouillé pour la transcription.

## Constraints and assumptions

- iOS termine une app qui envoie du travail Metal en arrière-plan : « A Metal
  app cannot execute Metal commands in the background, and a Metal app that
  attempts this is terminated. » Sources :
  <https://stackoverflow.com/questions/43680239>,
  <https://github.com/argmaxinc/WhisperKit/issues/194>. C'est exactement la
  pile des crash reports.
- `BGContinuedProcessingTask` (iOS 26+) permet de continuer en arrière-plan
  une tâche lancée par l'utilisateur, GPU compris sur les appareils
  compatibles :
  <https://developer.apple.com/documentation/backgroundtasks/performing-long-running-tasks-on-ios-and-ipados>.
  Des développeurs rapportent que `supportedResources` ne contient `.gpu`
  sur aucun iPhone testé (forum, non officiel) :
  <https://origin-devforums.apple.com/forums/thread/816774>. D'où le report
  hors périmètre.
- whisper-rs 0.16 expose `set_abort_callback_safe` : une inférence peut
  s'arrêter proprement entre deux étapes (vu dans le registre cargo).
- `sync_ffi.rs` observe déjà `UIApplicationDidEnterBackgroundNotification`
  et `UIApplicationWillEnterForegroundNotification` : le motif existe.
- Hypothèse à vérifier : arrêter Whisper à la réception de la notification
  d'arrière-plan laisse le dernier travail GPU déjà soumis se terminer sans
  abort. À prouver sur l'appareil (verrouiller en pleine transcription).
- Aujourd'hui l'audio entier (66 min à 48 kHz) est chargé en mémoire en une
  fois ; une reprise par morceaux le rend inutile.

## Boundary

Owns:
- `src/infrastructure/transcription/whisper.rs`
- `src/application/transcription_manager/`
- `src/infrastructure/persistence/pending_transcription_repo.rs`
- `src/ui/notes/audio_section.rs`
- `src/infrastructure/platform/ios/` (observateur de cycle de vie et
  maintien de l'écran)

Must not touch:
- `src/infrastructure/transcription/client.rs`
- `src/infrastructure/audio.rs`
- `src/infrastructure/sync/`
