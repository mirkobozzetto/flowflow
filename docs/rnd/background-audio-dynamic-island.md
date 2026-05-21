# Background Audio + Dynamic Island - Research & Prompts

Recherche du 2026-05-21. 4 taches independantes, chacune avec son prompt.

---

## Recherche : faits cles

- `UIBackgroundModes` absent → iOS suspend cpal stream en background
- cpal 0.17 = CoreAudio RemoteIO, respecte AVAudioSession config
- AVAudioSession actuel : PlayAndRecord + DefaultToSpeaker + AllowBluetoothA2DP
- Capture audio systeme (YouTube, appels) = bloque par Apple sur iOS. Mic uniquement.
- Dioxus 0.7 → 0.7.9 = semver-compatible, `cargo update` + `dx self-update`
- PR #4842 (Widget Extensions + Live Activities) = inclus depuis Dioxus 0.7.4
- Dynamic Island = ActivityKit + WidgetKit, SwiftUI obligatoire
- Dioxus #4709 : WKWebView crash apres long background, mais enregistrement actif garde l'app vivante
- WWDC25 : `bluetoothHighQualityRecording` pour AirPods (iOS 26, futur)

---

## Prompt 1 : Background Audio Recording

```
<context>
FlowFlow = app iOS 100% Rust, Dioxus 0.7, cpal 0.17.
Enregistrement audio coupe quand l'app passe en background.
Cause : UIBackgroundModes absent dans Dioxus.toml.
cpal utilise CoreAudio RemoteIO qui respecte AVAudioSession.
Si UIBackgroundModes audio declare + session active → stream continue en background.

Fichiers :
- Dioxus.toml : config Info.plist keys (section [ios.plist])
- src/platform/ios/mod.rs : configure_audio_session() avec AVAudioSession
  Options actuelles : DefaultToSpeaker | AllowBluetoothA2DP
  Category : PlayAndRecord
</context>

<task>
Activer l'enregistrement audio en background iOS.

1. Dans Dioxus.toml section [ios.plist], ajouter :
   UIBackgroundModes = ["audio"]

2. Dans src/platform/ios/mod.rs configure_audio_session(), ajouter MixWithOthers :
   options = DefaultToSpeaker | AllowBluetoothA2DP | MixWithOthers

3. Verifier que setActive(false) n'est JAMAIS appele pendant un enregistrement actif.
   Chercher tous les appels setActive dans le code.

4. make format && make check
</task>

<constraints>
- Zero comments dans le code
- Ne pas toucher a la state machine RecordingState
- Ne pas modifier le flux cpal
- Capture systeme impossible (mic uniquement), ne pas essayer
- Pas de commit sans approbation
</constraints>

<success_criteria>
- Enregistrer → switch vers Gmail → revenir → enregistrement a continue
- Enregistrer → YouTube joue en meme temps → les deux fonctionnent
- make check clean
</success_criteria>
```

---

## Prompt 2 : Interruption Handling (appels telephoniques)

```
<context>
FlowFlow = app iOS Rust/Dioxus.
L'enregistrement audio tourne en background (UIBackgroundModes audio).
Quand un appel telephonique arrive, iOS interrompt la session audio.
Sans handling, l'enregistrement est perdu.

Fichiers :
- src/platform/ios/mod.rs : AVAudioSession config, objc2 bindings
- src/services/audio.rs : AudioRecorder, RecordingState enum
  States : Idle, Recording, Paused, Transcribing, Transcribed(String), Error(String)
- src/ui/recording/controls.rs : UI controles enregistrement
- src/ui/state.rs : AppState avec recording_state: Signal<RecordingState>

Dependencies : objc2-avf-audio 0.3, objc2-foundation
</context>

<task>
Gerer les interruptions audio iOS (appels, FaceTime).

1. Dans src/platform/ios/mod.rs, observer AVAudioSessionInterruptionNotification :
   - Utiliser NSNotificationCenter::defaultCenter().addObserver
   - Sur InterruptionTypeBegan : exposer un callback/signal vers Rust
   - Sur InterruptionTypeEnded : verifier le flag shouldResume, exposer callback

2. Dans src/services/audio.rs :
   - Ajouter methode on_interruption_began() : pause le stream cpal, garder session active
   - Ajouter methode on_interruption_ended(should_resume: bool) :
     Si should_resume → reactiver session, rebuild stream cpal
     Si !should_resume → rester en pause

3. Connecter les callbacks iOS aux methodes AudioRecorder.
   Le bridge doit passer par un Arc<Mutex<AudioRecorder>> ou un channel.

4. L'UI (recording_state signal) doit refleter l'interruption :
   Recording → Paused (sur began)
   Paused → Recording (sur ended + shouldResume)

5. make format && make check
</task>

<constraints>
- Zero comments dans le code
- Ne pas changer la signature publique de RecordingState
- Le resume apres interruption doit etre automatique SI shouldResume = true
- Sinon l'utilisateur decide (bouton resume dans l'UI)
- Pas de commit sans approbation
</constraints>

<success_criteria>
- Enregistrer → recevoir appel → raccrocher → enregistrement reprend automatiquement
- Enregistrer → FaceTime → refuser → enregistrement reprend
- Si shouldResume = false → UI montre Paused, user peut reprendre manuellement
- make check clean
</success_criteria>
```

---

## Prompt 3 : Upgrade Dioxus 0.7 → 0.7.9

```
<context>
FlowFlow utilise Dioxus 0.7.0.
Dioxus 0.7.9 est la derniere version stable (2026-05-08).
0.7.0 → 0.7.9 = semver-compatible, pas de breaking changes.
dx CLI doit matcher la version.

Releases cles :
- 0.7.4 : iOS Widget Extensions (PR #4842), Swift FFI, Dioxus.toml iOS config
- 0.7.5 : fix futures dep (0.3.32 requis)
- 0.7.6 : dernier feature release 0.7
- 0.7.7-0.7.9 : macOS signing fix, iframe fix, dx fixes

Cargo.toml actuel : dioxus = { version = "0.7" }
dx CLI actuel : 0.7.7
</context>

<task>
Upgrade Dioxus vers 0.7.9.

1. cargo update (resout dioxus 0.7.9 automatiquement via semver range "0.7")
2. dx self-update (ou cargo install dioxus-cli@0.7.9 --force)
3. Verifier dx --version = 0.7.9
4. make build (cargo build --features mobile)
5. Si erreurs de compilation : analyser et fixer (probablement aucune)
6. make check (fmt + clippy)
7. Tester sur simulateur : make dev
8. Tester sur device : make ddev
</task>

<constraints>
- Ne pas sauter a 0.8.0-alpha (instable)
- Si un crate a un conflit de version, resoudre dans Cargo.toml
- Verifier que futures 0.3.32 est bien la version resolue (requis depuis 0.7.5)
- Pas de commit sans approbation
</constraints>

<success_criteria>
- cargo build --features mobile compile sans erreur
- dx --version = 0.7.9
- make check clean
- App fonctionne sur simulateur et device
</success_criteria>
```

---

## Prompt 4 : Dynamic Island (Live Activities)

```
<context>
FlowFlow = app iOS Rust/Dioxus 0.7.9+ (apres upgrade).
Dioxus 0.7.4+ supporte nativement les Widget Extensions via Dioxus.toml.
PR #4842 ajoute : ios.widget_extensions, pipeline FFI Swift, bundling automatique.

L'utilisateur veut voir le timer d'enregistrement dans le Dynamic Island
pendant que l'app enregistre en background.

Dynamic Island = framework ActivityKit + WidgetKit.
UI = SwiftUI obligatoire dans une Widget Extension (target separee).
4 layouts : compact leading, compact trailing, minimal, expanded.

Exemple reference : examples/01-app-demos/geolocation-native-plugin/ dans le repo Dioxus.

Fichiers existants :
- src/services/audio.rs : AudioRecorder avec start/pause/resume/stop/cancel
  duration_secs() retourne la duree actuelle
- src/ui/recording/controls.rs : UI controles, waveform 28 bars, update 60ms
- Dioxus.toml : config iOS
</context>

<task>
Implementer le Dynamic Island pour l'enregistrement audio.

1. Dioxus.toml : ajouter config Widget Extension
   ```toml
   [ios.plist]
   NSSupportsLiveActivities = true

   [[ios.widget_extensions]]
   source = "src/ios/widget"
   display_name = "FlowFlow Recording"
   bundle_id_suffix = "recording-widget"
   deployment_target = "16.2"
   module_name = "RecordingPlugin"
   ```

2. Creer src/ios/widget/RecordingAttributes.swift (~30 lignes) :
   - import ActivityKit
   - struct RecordingAttributes: ActivityAttributes
   - ContentState : elapsedSeconds (Int), isPaused (Bool)

3. Creer src/ios/widget/RecordingWidget.swift (~150 lignes) :
   - import WidgetKit, SwiftUI, ActivityKit
   - 4 layouts :
     Compact leading : icone micro rouge (circle fill)
     Compact trailing : timer MM:SS (Text avec .timer date style)
     Minimal : icone micro rouge petit
     Expanded : timer grand + icone + label "Enregistrement en cours"
   - Utiliser .contentTransition(.numericText()) pour animer le timer

4. Creer bridge FFI Rust (src/platform/ios/live_activity.rs) :
   - start_live_activity() : Activity<RecordingAttributes>.request()
   - update_live_activity(elapsed_secs: u32) : activity.update()
   - end_live_activity() : activity.end()
   - Utiliser manganis::ffi ou objc2 bindings

5. Brancher dans AudioRecorder :
   - start() → start_live_activity()
   - Timer 1s dans recording loop → update_live_activity(elapsed)
   - stop()/cancel() → end_live_activity()
   - pause() → update avec isPaused = true
   - resume() → update avec isPaused = false

6. make format && make check
7. Tester sur device physique (Dynamic Island = iPhone 14 Pro+ ou iPhone 15+)
</task>

<constraints>
- Widget Extension = Swift uniquement (pas de Rust dans le widget)
- Le code Rust du bridge utilise les FFI Dioxus (manganis::ffi)
- Zero comments dans le code (Rust ET Swift)
- Timer dans Dynamic Island : utiliser Text(.now, style: .timer) pour refresh automatique cote widget (pas besoin d'update chaque seconde depuis l'app)
- Pas de commit sans approbation
- Prerequis : Dioxus 0.7.4+ (faire Prompt 3 avant)
</constraints>

<success_criteria>
- Demarrer enregistrement → Dynamic Island affiche timer
- Passer en background → Dynamic Island toujours visible avec timer
- Pause → Dynamic Island montre "pause"
- Stop → Dynamic Island disparait
- Fonctionne sur iPhone 14 Pro+ physique
- make check clean
</success_criteria>
```

---

## Ordre d'execution

1. **Prompt 1** : Background audio (10 min, standalone)
2. **Prompt 2** : Interruption handling (1h, apres Prompt 1)
3. **Prompt 3** : Upgrade Dioxus (30 min, independant)
4. **Prompt 4** : Dynamic Island (1-2 jours, apres Prompt 3)
