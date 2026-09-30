---
type: brief
slug: background-local-transcription
title: Transcription locale qui continue écran éteint
status: ready
created: 2026-10-01
next_action: Une transcription Whisper locale continue d'avancer écran verrouillé, avec sa progression affichée par iOS.
resume_cmd: /ship docs/brief/background-local-transcription
base: dev
branch: feat/background-local-transcription
issues: "#198"
---

# Transcription locale qui continue écran éteint

## Problem

Depuis #197, une transcription Whisper locale survit au verrouillage de
l'écran, mais elle se met en pause. Pour un enregistrement d'une ou deux
heures, l'utilisateur doit garder l'app ouverte et l'écran allumé pendant
toute la transcription (environ deux fois le temps réel sur iPhone 16).
Dès qu'il range son téléphone, plus rien n'avance.

## Users

- L'utilisateur qui enregistre une réunion ou un entretien long, veut le
  texte sans envoyer l'audio dans le cloud, et range son téléphone ensuite.

## Goals

- Une transcription locale longue avance même écran verrouillé ou app
  quittée.
- L'utilisateur voit où elle en est sans rouvrir l'app.
- Le travail fait n'est jamais perdu, quel que soit le moteur utilisé.

## Acceptance criteria

- AC1 Verrouiller l'écran ou quitter l'app pendant une transcription
  locale lancée par l'utilisateur : la transcription continue d'avancer.
  Au retour, la note a plus de texte qu'au moment du verrouillage.
- AC2 Pendant ce temps, iOS affiche une activité en direct avec le titre
  de la note et le pourcentage d'avancement.
- AC3 Toucher « Arrêter » dans cette activité met la transcription en
  pause sans rien perdre ; elle reprend au retour dans l'app.
- AC4 Au retour dans l'app, la transcription repasse toute seule sur le
  moteur le plus rapide disponible au premier plan.
- AC5 Aucun rapport de crash `ggml_metal_synchronize` après un verrouillage
  en pleine transcription.
- AC6 Sur un iPhone sous iOS antérieur à 26, le comportement actuel est
  conservé : pause écran éteint, reprise au retour.
- AC7 Le texte final reste continu et toucher un mot saute au bon moment
  de l'audio, même quand les morceaux ont été traités par des moteurs
  différents.

## Success metrics

Sur l'iPhone de Mirko, une transcription de 66 min lancée puis écran
verrouillé 30 min a nettement avancé au retour, sans crash. Le rapport de
la mesure de T01 (vitesse en arrière-plan) est consigné dans l'issue #198.

## Out-of-scope

- Le GPU en arrière-plan : Apple ne l'accorde qu'aux iPad M3 et plus
  récents, aucun iPhone.
- La fiabilité des envois Soniox (#199).
- Les chapitres pour les longues transcriptions (#200).
- L'app Mac : pas de restriction GPU en arrière-plan.
- Android.

## Constraints and assumptions

- `BGContinuedProcessingTask` (iOS 26+) garde en vie une tâche lancée au
  premier plan par un geste de l'utilisateur, affiche sa progression dans
  une activité en direct et permet de l'annuler :
  <https://developer.apple.com/documentation/backgroundtasks/performing-long-running-tasks-on-ios-and-ipados>.
- Le GPU en arrière-plan est réservé aux iPad M3+, aucun iPhone :
  <https://developer.apple.com/forums/thread/801229>.
- L'autorisation « Background Inference » donne accès au Neural Engine en
  arrière-plan via Core ML :
  <https://apple-docs.everest.mt/docs/bundleresources/entitlements/com.apple.developer.background-tasks.continued-processing.inference/>.
  Son support sur iPhone 16 (iOS 27.2) n'est pas vérifié : T01 le mesure.
- whisper-rs 0.16 expose une feature `coreml` (encodeur sur Core ML) ; le
  build actuel n'active que `metal`. Elle demande un modèle d'encodeur
  Core ML en plus du modèle ggml (à vérifier dans T01).
- Hypothèse : le CPU seul est accepté même s'il est plus lent que le temps
  réel ; avancer vaut mieux qu'une pause. T01 mesure la vitesse réelle.
- La demande doit partir d'un geste de l'utilisateur au premier plan. Une
  reprise automatique au lancement de l'app n'en est pas un : la tâche de
  fond ne démarre alors qu'au prochain geste (flèche d'envoi, bouton
  retranscrire, ou reprise depuis la note).
- Déjà en place (#197) : `gpu_gate` suspend le GPU en arrière-plan,
  Whisper tourne par morceaux de 60 s et chaque morceau est sauvegardé.
  Changer de moteur se fait entre deux morceaux.
- Glue Swift : suivre `.claude/rules/ios-swift-glue.md` (ancre
  `@_cdecl` contre le dead-strip). L'identifiant de tâche est déclaré dans
  `BGTaskSchedulerPermittedIdentifiers`, préfixé par l'identifiant de
  l'app.

## Boundary

Owns:
- `src/infrastructure/transcription/whisper.rs`
- `src/infrastructure/transcription/gpu_gate.rs`
- `src/infrastructure/platform/ios/`
- `src/application/transcription_manager/`
- `src/ios/plugin/`
- `Dioxus.toml` (clés iOS de tâche de fond)

Must not touch:
- `src/infrastructure/transcription/client.rs`
- `src/infrastructure/sync/`
- `src/ui/notes/transcript_view.rs`
