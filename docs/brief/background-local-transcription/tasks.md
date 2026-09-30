---
type: tasks
slug: background-local-transcription
source_brief: docs/brief/background-local-transcription/brief.md
---

# Tasks: Transcription locale qui continue écran éteint

## Relevant Files

- `src/infrastructure/transcription/whisper.rs` - boucle par morceaux
  (`run_whisper`, `infer`), contexte Whisper mis en cache ; le choix du
  moteur se fait ici, entre deux morceaux.
- `src/infrastructure/transcription/gpu_gate.rs` - bloque le GPU en
  arrière-plan ; en arrière-plan, la boucle bascule au lieu d'attendre.
- `src/infrastructure/platform/ios/gpu_lifecycle.rs` - observateurs
  arrière-plan / premier plan.
- `src/application/transcription_manager/processing.rs` - `process_local`,
  progression et point de reprise.
- `src/ios/plugin/` - glue Swift pour `BGTaskScheduler`.
- `Dioxus.toml` - `background_modes`, clés Info.plist.

## Tasks

Ordered. Each task closes the acceptance criteria it names.

## T01 - Mesure sur l'iPhone : ce qu'iOS accorde en arrière-plan

Closes: aucun (décision consignée dans #198)

- [ ] `BGTaskScheduler.supportedResources` lu sur l'iPhone 16 (iOS 27.2),
      avec et sans l'autorisation Background Inference.
- [ ] Vitesse d'un morceau de 60 s en arrière-plan mesurée sur CPU, et sur
      Neural Engine si accordé.
- [ ] Moteur retenu pour l'arrière-plan écrit dans #198, avec les chiffres.

## T02 - La transcription continue écran verrouillé

Closes: AC1, AC4, AC5, AC7

- [ ] Verrouiller l'écran en pleine transcription : elle avance sur le
      moteur retenu en T01, sans crash.
- [ ] Au retour, elle repasse seule sur le GPU au morceau suivant.
- [ ] Mots avant, pendant et après l'arrière-plan : texte continu, toucher
      un mot saute au bon moment.

## T03 - Progression et arrêt depuis l'activité iOS

Closes: AC2, AC3

- [ ] L'activité en direct d'iOS montre le titre de la note et le
      pourcentage, qui progresse.
- [ ] « Arrêter » met en pause sans rien perdre ; reprise au retour.

## T04 - Anciennes versions d'iOS inchangées

Closes: AC6

- [ ] Sous iOS antérieur à 26, pause écran éteint et reprise au retour,
      comme aujourd'hui.

## T05 - 66 min écran verrouillé sur l'iPhone

Closes: AC1, AC2, AC5

- [ ] Lancer une transcription de 66 min, verrouiller 30 min : elle a
      nettement avancé, aucun nouveau rapport de crash.
