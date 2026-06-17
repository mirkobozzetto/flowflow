# System audio capture & audio import — research

> Target: 100% Rust + Dioxus iOS app (FlowFlow), voice notes + transcription
> Scope: capturing system audio, recording calls, headphone routing, file import, Soniox format support
> Date: 2026-05-23
> Status: research only, no implementation

## TL;DR

| Question | Verdict | Reason |
|----------|---------|--------|
| Capture YouTube/Spotify audio while recording mic | **Yes, but only via Broadcast Upload Extension (ReplayKit)** with hard limits | Blocked for AVPlayer/Music/Safari/DRM content. Requires app extension + App Groups + 50 MB memory cap |
| Capture WhatsApp/FaceTime/phone calls | **Hard NO** | OS-level block, not policy. Workaround only via speakerphone + mic (lossy, no separation) |
| AirPods routing while playing another app's audio | **Yes if A2DP for output, built-in mic for input** | `.allowBluetoothA2DP` (NOT `.allowBluetooth`); HFP downgrades quality and locks I/O together |
| Audio file import (m4a/mp3/wav/aac) | **Yes, fully supported** | `UIDocumentPickerViewController` with `UTType.audio`; also Share Extension + App Group for Voice Memos / AirDrop |
| Soniox supported formats | **All iOS native formats work** | Auto-detect for aac/aiff/amr/asf/flac/mp3/ogg/wav/webm + m4a/mp4 (async) — no conversion needed |

### Recommendation for FlowFlow

- **MVP**: skip system audio capture. Engineering cost very high, narrow value. Use mic-only `playAndRecord` + `mixWithOthers` so podcasts continue playing in the background. **Already in place in `src/platform/ios/mod.rs`.**
- **Phase 2 (if needed)**: add a Broadcast Upload Extension (Swift, signed separately) that writes captured audio to an App Group container; Rust app transcribes the resulting `.m4a` via Soniox.
- **Calls**: don't attempt. Document the limitation in the UI.
- **Import**: extend the existing `open_file_picker` in `src/platform/ios/picker.rs` to handle `public.audio`; push the resulting `URL` straight to Soniox as `audio_url` or upload the file body — no decode needed in Rust.

### FlowFlow current state (verified)

`src/platform/ios/mod.rs::configure_audio_session` already uses:
- Category: `playAndRecord`
- Options: `DefaultToSpeaker | AllowBluetoothA2DP | MixWithOthers`
- Interruption observer wired via `NSNotificationCenter` (`AVAudioSessionInterruptionNotification`)

→ AVAudioSession setup aligns with research best practices. No change required.

---

## 1. Capturing system audio while recording the mic

### 1.1 What `AVAudioSession` can and cannot do

`AVAudioSession` controls **your app's** audio relationship with the system. It does **not** give access to other apps' audio buffers. The `MixWithOthers` option only controls whether your session **interrupts** other apps' playback. It does not route their audio into your input stream.

- `playAndRecord` is the only category that allows simultaneous mic input + speaker output (Apple Audio Session Programming Guide). `record` alone "silences virtually all system output and is usually too restrictive". Source: [Apple AudioGuidelinesByAppType](https://developer.apple.com/library/archive/documentation/Audio/Conceptual/AudioSessionProgrammingGuide/AudioGuidelinesByAppType/AudioGuidelinesByAppType.html).
- Adding `.mixWithOthers` option lets background music keep playing **while your app is alive but not recording**. As soon as you start the AVAudioEngine input node (or AVAudioRecorder), iOS will pause or duck the other audio. Confirmed: [Stack Overflow — Setting Audio Input node for AVAudioEngine causes outside audio to stop](https://stackoverflow.com/questions/78790511/setting-audio-input-node-for-avaudioengine-causes-outside-audio-to-stop).
- `.duckOthers` lowers other audio volume, doesn't capture it.
- `.interruptSpokenAudioAndMixWithOthers` only mixes with non-spoken audio.
- `mixWithOthers` reference: [Apple Developer — mixWithOthers](https://developer.apple.com/documentation/avfaudio/avaudiosession/categoryoptions-swift.struct/mixwithothers).

**Bottom line**: `AVAudioSession` is the wrong API for capturing other apps' audio. It only governs whether your app cohabitates with others.

### 1.2 What ReplayKit can do (the only path)

`ReplayKit` is Apple's only API that exposes other apps' audio. Two modes:

| Mode | Scope | Audio captured | Where it runs |
|------|-------|---------------|---------------|
| **In-app capture** (`RPScreenRecorder.startCapture`) | Only YOUR app's own UI/audio | Mic + your app's audio | Main app process |
| **Broadcast Upload Extension** (`RPBroadcastSampleHandler`) | System-wide (all apps + home screen) | Mic + system app audio (with carve-outs) | Separate extension process (50 MB cap) |

Sources:
- [Apple — RPBroadcastSampleHandler](https://developer.apple.com/documentation/replaykit/rpbroadcastsamplehandler)
- [Apple ReplayKit framework page](https://developer.apple.com/documentation/ReplayKit)
- [WWDC 2017 session 606 "What's New with Screen Recording and Live Broadcast"](https://nonstrict.eu/wwdcindex/wwdc2017/606/)
- [WWDC 2020 session 10633 (macOS ReplayKit)](https://developer.apple.com/videos/play/wwdc2020/10633/)
- [WWDC 2021 session 10101 (clips recording)](https://developer.apple.com/videos/play/wwdc2021/10101/)

The Broadcast Upload Extension hands you three sample-buffer types via `processSampleBuffer(_:with:)`:
- `RPSampleBufferType.audioApp` — combined audio output of all other apps
- `RPSampleBufferType.audioMic` — microphone
- `RPSampleBufferType.video` — screen frames (you can ignore for audio-only use)

Reference enum: [RPSampleBufferType](https://apidocs.mobileui.dev/robovm/latest/org/robovm/apple/replaykit/RPSampleBufferType.html).

### 1.3 Hard limits Apple imposes

From the [Twilio ReplayKit example README](https://github.com/twilio/video-quickstart-ios/blob/master/ReplayKitExample/README.md) (most thorough field report):

1. **DRM and Apple-internal apps are silently muted**:
   > "It is not possible to capture application audio produced by `AVPlayer`, by Safari video playback (even if no FairPlay DRM is used), or by the Music app."

   So Apple Music, Apple TV, Netflix, Spotify, and Safari `<video>` playback all return **silence buffers**. Confirmed by Apple forum thread [WKWebView + ReplayKit silence](https://developer.apple.com/forums/thread/670482).

2. **50 MB memory cap on the extension** — extension is killed if exceeded. No Audio Unit graph, no playback inside the extension, downscale video, single-thread audio. Sources: Twilio README, [OpenTok iOS SDK Broadcast-Ext](https://github.com/opentok/opentok-ios-sdk-samples/blob/main/Broadcast-Ext/README.md).

3. **Out-of-process security model**: the broadcast extension runs in Apple's `replayd` daemon and is sandboxed from your main app. To get audio into your main app you need an **App Group** + shared container (`UserDefaults(suiteName:)` for signalling, `FileManager.containerURL(forSecurityApplicationGroupIdentifier:)` for files). [LiveKit ios-screen-sharing.md](https://github.com/livekit/client-sdk-swift/blob/main/Docs/ios-screen-sharing.md), [100ms-docs screen-share](https://github.com/100mslive/100ms-docs/blob/main/docs/ios/v2/how-to-guides/set-up-video-conferencing/screen-share.mdx), [Apple security guide for ReplayKit](https://support.apple.com/guide/security/replaykit-security-seca5fc039dd/web).

4. **User-initiated start only**: you cannot programmatically start a broadcast. You must present `RPSystemBroadcastPickerView` and the user taps "Start Broadcast" inside the system sheet. Reference: [RPSystemBroadcastPickerView (Apple)](https://developer.apple.com/documentation/replaykit/rpsystembroadcastpickerview?changes=__2&language=objc).

5. **iOS 13.0 had a memory leak** when `audioMic` was enabled; fixed in 13.1. Modern iOS 18+/26 is fine.

### 1.4 Audio sample format from the extension (post iOS 13)

From the Twilio README field measurements:

| Buffer type | Channels | Sample rate | Buffer size | Period |
|-------------|----------|-------------|-------------|--------|
| `audioApp`  | 1 or 2   | 44 100 Hz   | 1024 frames | 23.2 ms |
| `audioMic`  | 1        | 44 100 Hz   | 1024 frames | 23.2 ms |

Note byte-order difference: app audio is big-endian, mic audio is little-endian.

### 1.5 Implementation hints for FlowFlow

A Broadcast Upload Extension is an **Xcode target written in Swift/Objective-C with App Extension API restrictions**. You cannot replace it with a pure-Rust target. Minimal viable path:

1. Add a new Xcode target `FlowFlowBroadcast` of type **Broadcast Upload Extension** (Swift). Inside `SampleHandler.swift`, write audio buffers (mic + app) to an `AVAssetWriter` whose output URL is in the App Group container.
2. Add the **App Groups** capability to both the main app target and the extension with the same group ID (e.g. `group.com.mirkobozzetto.flowflow`).
3. Set bundle ID of extension to `com.mirkobozzetto.flowflow.broadcast`.
4. In FlowFlow's main Rust process, expose `RPSystemBroadcastPickerView` via a thin Swift bridge (or via `objc2` msg_send like FlowFlow already does for the iOS picker). Set its `preferredExtension = "com.mirkobozzetto.flowflow.broadcast"`.
5. After broadcast stops, Rust reads the `.m4a` from the shared container via `FileManager.containerURL(forSecurityApplicationGroupIdentifier:)` (via `objc2` msg_send), copies it to `documents_dir()`, then runs the existing Soniox pipeline.
6. Mind the **50 MB cap** — don't buffer raw PCM; write directly to AAC via `AVAssetWriter`.
7. Update Makefile: extension needs its own provisioning profile, signed alongside the main app. Same complexity as your current Live Activity Widget Extension plan — extend that pattern.

### 1.6 ReplayKit references

- Apple: [ReplayKit framework](https://developer.apple.com/documentation/ReplayKit)
- Apple Security: [ReplayKit Security in iOS](https://support.apple.com/guide/security/replaykit-security-seca5fc039dd/web)
- Apple: [RPBroadcastSampleHandler](https://developer.apple.com/documentation/replaykit/rpbroadcastsamplehandler)
- Twilio: [ReplayKitExample README — gotchas, formats, limitations](https://github.com/twilio/video-quickstart-ios/blob/master/ReplayKitExample/README.md)
- LiveKit: [iOS screen sharing — app group + broadcast capture](https://github.com/livekit/client-sdk-swift/blob/main/Docs/ios-screen-sharing.md)
- 100ms: [iOS screen share with broadcast extension](https://github.com/100mslive/100ms-docs/blob/main/docs/ios/v2/how-to-guides/set-up-video-conferencing/screen-share.mdx)
- OpenTok: [Broadcast-Ext sample](https://github.com/opentok/opentok-ios-sdk-samples/blob/main/Broadcast-Ext/README.md)
- WWDC 2017: [Session 606 — In-App Screen Capture intro](https://nonstrict.eu/wwdcindex/wwdc2017/606/)

---

## 2. Recording VoIP / calls (WhatsApp, FaceTime, phone)

### 2.1 The blanket NO

> "On iOS, Apple's security framework is particularly restrictive. The company doesn't allow any third-party app to access the audio stream during VoIP calls made through applications like WhatsApp, Telegram, or FaceTime. This isn't a limitation that developers can work around; it's a hard restriction built into the operating system's architecture."
> — [Remi8 — Why WhatsApp Call Recording Apps Fail](https://remi8.ai/blog/whatsapp-call-3/why-whatsapp-call-recording-apps-fail-16)

### 2.2 Why it can't be worked around

- **CallKit audio session is privileged**. From Apple engineer (quintonn) on Apple Forums [thread 758386](https://developer.apple.com/forums/thread/758386):
  > "The audio session category CallKit activates is NOT a 'standard' AVAudioSessionCategoryPlayAndRecord category, but a 'special' phone specific session ... it has higher activation priority than EVERY other audio session type on the system and CANNOT be interrupted by any of the public session categories."
- While a CallKit-managed call is active, any other app that tries to grab the microphone is either denied or causes the call to end. ReplayKit's `audioApp` stream is also blocked from CallKit audio for the same security reason (no Apple doc spells this out explicitly, but every implementer reports silence).
- iOS does **not** expose call logs/history to third parties at all. Forum [thread 764958](https://developer.apple.com/forums/thread/764958):
  > "There isn't any API on the system which will give you access to the user call history ... Developers have been asking for access to the user call history since iOS was first introduced and, at this point, it should be fairly clear that this simply isn't something iOS is going to provide."

### 2.3 Apple's own (legal) call recording

Apple ships native call recording **only in the Phone app** (cellular calls, not VoIP), iOS 18+, and **not in the EU** or many other regions. The recipient is forced to hear an audio notice. Reference: [Apple Support — Record and transcribe a call on iPhone](https://support.apple.com/guide/iphone/record-and-transcribe-a-call-iph57c6590e9/ios).

You cannot replicate this — Apple uses private system APIs and a legal disclosure that only Apple is allowed to ship.

### 2.4 The only workaround: speakerphone + open mic

Put the call on speakerphone, run FlowFlow's normal mic recorder. You get a single mono track that captures whatever the speaker emits + room reflections + your own voice + ambient noise. Drawbacks:
- Speaker quality is poor
- No diarisation between caller/callee (Soniox `enable_speaker_diarization: true` won't reliably split since acoustic separation is weak)
- Echo/cross-talk
- Many regions require consent of both parties (legal exposure)

For FlowFlow this is not worth shipping as a "call recorder" feature. Better to surface a note in-app: "FlowFlow records your microphone — for calls, use Apple's native call recording where available."

### 2.5 Call recording references

- [Remi8 — Why WhatsApp Call Recording Apps Fail](https://remi8.ai/blog/whatsapp-call-3/why-whatsapp-call-recording-apps-fail-16)
- Apple Dev Forum thread 758386 — [CallKit audio session priority](https://developer.apple.com/forums/thread/758386)
- Apple Dev Forum thread 774784 — [WebRTC voice calls in background limitations](https://developer.apple.com/forums/thread/774784)
- Apple Dev Forum thread 764958 — [No call log API on iOS](https://developer.apple.com/forums/thread/764958)
- [Apple Support — Record and transcribe a call](https://support.apple.com/guide/iphone/record-and-transcribe-a-call-iph57c6590e9/ios)
- [WWDC 2016 session 230 — CallKit](https://developer.apple.com/videos/play/wwdc2016/230/)
- [Medium — How to Detect and Handle VoIP Calls on iOS Using CallKit](https://medium.com/swiftfy/how-to-detect-and-handle-voip-calls-on-ios-using-callkit-e32dab00933c)

---

## 3. Headphones / Bluetooth routing during recording

### 3.1 The Bluetooth profile trap

Bluetooth audio uses two incompatible profiles:
- **A2DP** — high-bandwidth stereo, **playback only**, no mic. Codecs: SBC, AAC, aptX.
- **HFP** (Hands-Free Profile) — bidirectional, **low-bandwidth mono**, used for phone calls. Sample rate typically 8 kHz (narrowband) or 16 kHz (wideband mSBC). Quality is "tinny, lacks bass".

When iOS opens a Bluetooth mic via HFP, the same Bluetooth link drops out of A2DP and downgrades the output. Sources: [Microsoft — Bluetooth Classic audio accessories](https://learn.microsoft.com/en-us/windows-hardware/design/accessory-guidelines/bluetooth-accessory-guidelines/bluetooth-accessory-guidelines-classic-audio), [Buralog — AirPods in web conferencing](https://buralog.jp/en/airpods-webconf-regret-en/), Apple Q&A QA1799 below.

### 3.2 The right option: `.allowBluetoothA2DP`, NOT `.allowBluetooth`

The naming in `AVAudioSession.CategoryOptions` is deeply misleading:

| Option | What it actually does |
|--------|----------------------|
| `.allowBluetooth` | Prefer **HFP** (telephony, low quality, bidirectional) |
| `.allowBluetoothA2DP` | Prefer **A2DP** (stereo high quality, output-only); mic falls back to built-in |
| `.allowAirPlay` | AirPlay routing |

Field guidance from Stack Overflow ([Recording from Built-In Mic when Playing through Bluetooth](https://stackoverflow.com/questions/30614134/recording-from-built-in-mic-when-playing-through-bluetooth-in-ios)):
> "By removing `.allowBluetooth` from AVAudioSession's `categoryOptions`, it won't allow HFP, which is a protocol to use bluetooth device as an input. Thus it will automatically change its input route to built-in mic."

And [Stack Overflow — How to use internal mic for input and bluetooth for output](https://stackoverflow.com/questions/65571861/how-to-use-internal-mic-for-input-and-bluetooth-for-output):
> "Get rid of `allowBluetooth` and use `allowBluetoothA2DP`. You also don't want `defaultToSpeaker` here. `Allow Bluetooth` actually means `prefer HFP`."

### 3.3 The locked I/O rule

Apple QA1799 ([Technical Q&A — AVAudioSession Microphone Selection](https://developer.apple.com/library/archive/qa/qa1799/_index.html)) and Apple Developer Forums [thread 4340](https://developer.apple.com/forums/thread/4340?answerId=15178022):

> "If an application uses the `setPreferredInput:error:` method to select a Bluetooth HFP input, the output will automatically be changed to the Bluetooth HFP output. Moreover, selecting a Bluetooth HFP output using the MPVolumeView's route picker will automatically change the input to the Bluetooth HFP input. Therefore both the input and output will always end up on the Bluetooth HFP device even though only the input or output was set individually."

In other words: **once HFP is engaged, input and output are bound together to the same Bluetooth device**. You cannot have "A2DP out + Bluetooth HFP in" or "A2DP out + built-in mic in over HFP". The hardware doesn't physically support it on the same link.

### 3.4 Will the AirPods mic hear audio playing from another app?

Scenarios:

| Scenario | Result |
|----------|--------|
| User wears AirPods, plays Spotify, FlowFlow uses `playAndRecord` + `.allowBluetoothA2DP` | Spotify keeps using A2DP. AirPods mic is disabled. FlowFlow records from **built-in iPhone microphone**. Built-in mic does NOT pick up the AirPods speaker output (it's sealed in user's ears). Result: clean voice recording, no leakage of Spotify into the recording. |
| Same but `.allowBluetooth` (HFP) | iOS switches AirPods to HFP. A2DP drops, so Spotify output becomes HFP-quality (terrible) or pauses. Mic is the AirPods mic — captures voice well at close range but doesn't pick up Spotify audio (Spotify is being routed *to* the same headset). |
| Wired EarPods (Lightning/3.5mm headset) playing music + FlowFlow records | EarPods have a separate mic on the cable. iOS routes input to `AVAudioSessionPortHeadsetMic`. Music playing through EarPods speakers does not leak into the headset mic (acoustically isolated from the in-ear driver). Result: clean voice. |
| iPhone on speakerphone playing podcast + FlowFlow records via built-in mic | Built-in mic captures both the user's voice **and** the podcast playing from the iPhone's speaker (poor SNR, no separation). This is the only "free" path to record system audio + voice — at terrible quality. |
| External wired headset (USB-C, EarPods) playing media + recording from built-in mic | Possible by using `setPreferredInput(builtInMic)` to override the route. Built-in mic still doesn't hear headphones (sealed). |

So the AirPods don't act as a Shazam-style ambient sniffer — they isolate output from input by design. Only acoustic leakage from a loudspeaker can land in the recording.

### 3.5 Route change handling

When the user plugs/unplugs headphones during a recording, you must observe `AVAudioSessionRouteChangeNotification`. Apple guide: [Responding to Route Changes](https://developer.apple.com/library/archive/documentation/Audio/Conceptual/AudioSessionProgrammingGuide/HandlingAudioHardwareRouteChanges/HandlingAudioHardwareRouteChanges.html).

For FlowFlow with `cpal`, the equivalent is to listen via an Objective-C bridge (`NotificationCenter.default.addObserver(forName: AVAudioSession.routeChangeNotification ...)`). On `oldDeviceUnavailable`, decide whether to pause the recording (Apple recommends pause on unplug for playback; for recording it's safer to continue, since users often pull out earbuds while a recording continues).

### 3.6 Recommended FlowFlow settings (already in place)

For the existing FlowFlow recording flow:

```text
Initial state (app foreground, no recording):
  setCategory(.playAndRecord,
              mode: .default,
              options: [.allowBluetoothA2DP, .defaultToSpeaker, .mixWithOthers])

When user taps record:
  setCategory(.playAndRecord,
              mode: .measurement OR .default,
              options: [.allowBluetoothA2DP, .defaultToSpeaker])
  (drop .mixWithOthers so other apps are interrupted/ducked while recording)

After recording stops:
  Restore initial state.
```

Note: `mode: .measurement` disables the input AGC and processing — better for transcription accuracy at the cost of perceived loudness. Test both.

**FlowFlow status**: initial state already correct. Could add the mid-recording toggle (drop `MixWithOthers`) but currently iOS will duck other audio automatically when the cpal stream activates.

### 3.7 Routing references

- Apple Q&A QA1799: [AVAudioSession Microphone Selection](https://developer.apple.com/library/archive/qa/qa1799/_index.html)
- Apple: [Configuring an Audio Session](https://developer.apple.com/library/archive/documentation/Audio/Conceptual/AudioSessionProgrammingGuide/AudioSessionBasics/AudioSessionBasics.html)
- Apple: [Responding to Route Changes](https://developer.apple.com/library/archive/documentation/Audio/Conceptual/AudioSessionProgrammingGuide/HandlingAudioHardwareRouteChanges/HandlingAudioHardwareRouteChanges.html)
- Apple Forums: [Bluetooth with AVAudioSessionPlayAndRecord](https://developer.apple.com/forums/thread/4340?answerId=15178022)
- Stack Overflow: [Recording from Built-In Mic when Playing through Bluetooth](https://stackoverflow.com/questions/30614134/recording-from-built-in-mic-when-playing-through-bluetooth-in-ios)
- Stack Overflow: [Use internal mic for input and bluetooth for output](https://stackoverflow.com/questions/65571861/how-to-use-internal-mic-for-input-and-bluetooth-for-output)
- Stack Overflow: [AVAudioSession and AirPods](https://stackoverflow.com/questions/47816753/avaudiosession-and-airpods)
- Microsoft Learn: [Bluetooth A2DP/HFP accessory guidelines](https://learn.microsoft.com/en-us/windows-hardware/design/accessory-guidelines/bluetooth-accessory-guidelines/bluetooth-accessory-guidelines-classic-audio)

---

## 4. Importing audio files from the device

### 4.1 The canonical API: `UIDocumentPickerViewController`

For everything that lives in Files.app (Local, iCloud Drive, On My iPhone, third-party providers like Dropbox/Google Drive):

```swift
import UniformTypeIdentifiers
let picker = UIDocumentPickerViewController(
    forOpeningContentTypes: [UTType.audio],
    asCopy: true
)
picker.delegate = self
picker.allowsMultipleSelection = false
present(picker, animated: true)
```

`UTType.audio` (the generic conformance) covers `mp3`, `m4a`, `wav`, `aac`, `aiff`, `caf`, `flac`, `opus`, `ogg` and other audio UTIs. The relevant UTIs: `public.mp3`, `com.apple.m4a-audio`, `com.microsoft.waveform-audio`, `public.aac-audio`, `public.aiff-audio`, `org.xiph.flac`, `org.xiph.ogg-audio`.

For a narrower picker, pass an array, e.g. `[UTType.mp3, UTType.wav, UTType.mpeg4Audio]`.

References:
- Apple: [UIDocumentPickerViewController](https://developer.apple.com/documentation/uikit/uidocumentpickerviewcontroller)
- Apple: [documentPicker(_:didPickDocumentsAt:)](https://developer.apple.com/documentation/uikit/uidocumentpickerdelegate/documentpicker(_:didpickdocumentsat:))
- Stack Overflow: [Pick audio files from device library](https://stackoverflow.com/questions/47791284/how-to-pick-browse-audio-files-from-device-library-like-photo-library)
- Stack Overflow: [Using UIDocumentPickerViewController(documentTypes:) in Swift (deprecation note)](https://stackoverflow.com/questions/66003954/using-uidocumentpickerviewcontrollerdocumenttypes-in-swift)
- Programming-iOS-Book-Examples: [PickACloudSong example](https://github.com/mattneub/Programming-iOS-Book-Examples/blob/master/bk2ch23p803PickACloudSong/PickACloudSong/ViewController.swift)

### 4.2 Security-scoped URLs (mandatory)

When `asCopy: false`, picked URLs are security-scoped — you must wrap reads:

```swift
guard url.startAccessingSecurityScopedResource() else { return }
defer { url.stopAccessingSecurityScopedResource() }
let data = try Data(contentsOf: url)
```

When `asCopy: true`, iOS copies the file into your app's `tmp/...Inbox/` directory and the URL is local and non-scoped. Persist by moving to `Documents/`.

FlowFlow's `read_file_as_text` in `src/platform/ios/picker.rs` already uses this pattern for text/PDF/DOCX. Add `asCopy: true` for audio and read the file bytes into Rust via `std::fs::read`.

### 4.3 Voice Memos integration

Voice Memos exports `.m4a` (AAC 64–128 kbps) via:
- **Share sheet** ("Save to Files" → user picks destination; FlowFlow then picks it back via `UIDocumentPickerViewController`).
- **AirDrop** to another device.
- **Share Extension hosted by FlowFlow** — FlowFlow would advertise itself in the Voice Memos share sheet via an Action/Share Extension declaring `NSExtensionActivationRule` with `NSExtensionActivationSupportsFileWithMaxCount > 0` and audio UTIs.

Apple docs:
- [Share a recording in Voice Memos](https://support.apple.com/en-am/guide/iphone/iph3d6dc359/26/ios/26)
- [Export a Voice Memos recording to Files](https://support.apple.com/en-nz/guide/iphone/iph831c37815/ios) — "Recordings are exported in .m4a format. Layered recordings are flattened and Spatial Audio becomes stereo."

Field examples for share-extension audio receivers:
- [Stack Overflow — Export audio files via Open In from Voice Memos](https://stackoverflow.com/questions/36763390/export-audiofiles-via-open-in-from-voice-memos-app)
- [expo-audio-share-receiver — Share Extension that drops audio in App Group, deep-links to host](https://github.com/OKKHALIL3/expo-audio-share-receiver)
- [Apple Forum — Extracting Updated Title from Voice Memos via Share Extension](https://developer.apple.com/forums/thread/744419)

For FlowFlow MVP, the share-extension route is optional. The picker route already covers Voice Memos because the user can do "Share → Save to Files" once and then import — only 2 taps extra and zero extra native code.

### 4.4 AirDrop received files

When a user accepts an audio file via AirDrop and your app is registered for that UTI in `Info.plist` (`CFBundleDocumentTypes` with `LSItemContentTypes = ["public.audio"]`), iOS shows your app in the AirDrop receiver chooser. The file lands in `Documents/Inbox/`.

### 4.5 What about MPMediaPickerController?

`MPMediaPickerController` only browses the Apple Music / iTunes library (DRM-protected items). Selected items don't always yield a usable local file path — protected media is unreachable. For FlowFlow, **stay on `UIDocumentPickerViewController`** which sees user-imported files and Files.app providers.

### 4.6 Implementation hints for FlowFlow

In `src/platform/ios/picker.rs` extend the picker with an audio variant. The objc2 call:

```text
NSArray<UTType*> *types = @[UTType.audio];
UIDocumentPickerViewController *picker = [[UIDocumentPickerViewController alloc]
  initForOpeningContentTypes:types
  asCopy:YES];
picker.delegate = self;
// present from key window's rootViewController
```

The picked URL is already a real file path in your sandbox — no security scope dance needed when `asCopy:YES`. Pass it to:

```rust
let bytes = std::fs::read(&picked_path)?;
// hand to Soniox SonioxClient::transcribe (audio_url upload or multipart)
```

Add the UTI claim to `Info.plist` (`CFBundleDocumentTypes`) so users can also "Open in FlowFlow" from Files/Mail/AirDrop:

```xml
<key>CFBundleDocumentTypes</key>
<array>
  <dict>
    <key>CFBundleTypeName</key>
    <string>Audio File</string>
    <key>LSHandlerRank</key>
    <string>Alternate</string>
    <key>LSItemContentTypes</key>
    <array>
      <string>public.audio</string>
      <string>public.mp3</string>
      <string>com.apple.m4a-audio</string>
      <string>com.microsoft.waveform-audio</string>
      <string>public.aac-audio</string>
    </array>
  </dict>
</array>
```

### 4.7 Import references (summary)

- [Apple — UIDocumentPickerViewController](https://developer.apple.com/documentation/uikit/uidocumentpickerviewcontroller)
- [Apple — documentPicker(_:didPickDocumentsAt:)](https://developer.apple.com/documentation/uikit/uidocumentpickerdelegate/documentpicker(_:didpickdocumentsat:))
- [Apple — Voice Memos sharing](https://support.apple.com/en-am/guide/iphone/iph3d6dc359/26/ios/26)
- [Apple — Export Voice Memos to Files](https://support.apple.com/en-nz/guide/iphone/iph831c37815/ios)
- [Programmer AH — SwiftUI `.fileImporter` with audio](https://programmerah.com/swiftui-2-0-how-to-import-files-into-ios-apps-26846/)
- [Stack Overflow — Pick audio file with UIDocumentPickerViewController](https://stackoverflow.com/questions/68070850/how-to-play-and-save-audiofile-from-filepath-ios-swift)
- [Stack Overflow — Voice Memos share extension](https://stackoverflow.com/questions/36763390/export-audiofiles-via-open-in-from-voice-memos-app)

---

## 5. Soniox audio format support

### 5.1 Async (file) transcription — used by FlowFlow

Soniox auto-detects everything iOS records or imports:

> "Soniox automatically detects audio formats for file transcription — no configuration required.
> Supported formats: `aac, aiff, amr, asf, flac, mp3, ogg, wav, webm, m4a, mp4`"
> — [Soniox docs — Async transcription](https://soniox.com/docs/stt/async/async-transcription)

So a file picked via `UIDocumentPickerViewController` or recorded as `.wav` by `hound` (FlowFlow's current pipeline) or as `.m4a` from Voice Memos goes straight into the `transcriptions` REST endpoint without conversion.

### 5.2 Real-time (streaming) transcription

Same auto-detect, slightly narrower:

> "Supported auto formats: `aac, aiff, amr, asf, flac, mp3, ogg, wav, webm`"
> Raw formats requiring sample rate + channels: `pcm_s8`, `pcm_s16le/be`, `pcm_s24le/be`, `pcm_s32le/be`, unsigned variants, `pcm_f32`, `pcm_f64`, `mulaw`, `alaw`.
> — [Soniox docs — Real-time](https://soniox.com/docs/stt/rt/real-time-transcription)

If FlowFlow ever streams from cpal directly (skipping the WAV step), the right config:
```json
{"audio_format": "pcm_s16le", "sample_rate": 16000, "num_channels": 1}
```
cpal on iOS typically gives `f32` at 44.1 kHz from the built-in mic; convert to `s16le` at 16 kHz with linear resampling (or just send `pcm_f32le` at 44100). Soniox accepts both.

### 5.3 API reference

- REST API at `https://api.soniox.com/v1`. Endpoints: Files (upload), Transcriptions (create / get).
- [Soniox API reference index](https://soniox.com/docs/api-reference)
- [Create transcription](https://soniox.com/docs/api-reference/stt/transcriptions/create_transcription) — supports `file_id` OR `audio_url`.
- [WebSocket API for real-time](https://soniox.com/docs/api-reference/stt/websocket-api).

### 5.4 Practical iOS recording format → Soniox

FlowFlow today: `cpal` + `hound` writes WAV (LPCM 16-bit). That's `wav` — directly transcribable, no conversion. Size is the only downside (~1.4 MB/min at 16 kHz mono).

If you want to shrink uploads ~10× and battery cost, switch the iOS recorder to **AVAudioRecorder with AAC in an .m4a container**. AAC at 64 kbps mono = ~480 KB/min, still excellent Soniox accuracy. AVAudioRecorder writes the container correctly; no Rust encoder needed.

Reference Apple types for m4a: `AVFileType.m4a` (UTI `com.apple.m4a-audio`).

### 5.5 Do you need any Rust audio conversion?

Probably **no** for FlowFlow's pipeline:
- Mic capture → `.wav` (current) → Soniox WAV path: works.
- Audio import → m4a/mp3/etc. → Soniox auto-detect: works.

If you ever do need conversion in Rust (e.g. trim, normalize, re-encode), the right crates are:
- **[Symphonia](https://github.com/pdeljanov/Symphonia)** (pure Rust, no C deps, iOS-friendly) — decodes m4a/aac, mp3, flac, wav, ogg, mkv. **Decode only**.
- **hound** — already in FlowFlow — encode/decode WAV (PCM only).
- **dasp** — sample type conversion (i16 ↔ f32) + resampling.
- For AAC/MP3 **encoding** you need C bindings (`fdk-aac-sys`, `lame-sys`, `ffmpeg-sys-next`) — painful on iOS cross-compile. Avoid: re-encode via AVFoundation in a Swift bridge instead.

Symphonia confirms iOS support explicitly: ["Mobile: Android, iOS"](https://pdeljanov-symphonia.mintlify.app/resources/faq).

### 5.6 Soniox references

- [Soniox — Async transcription](https://soniox.com/docs/stt/async/async-transcription)
- [Soniox — Real-time transcription](https://soniox.com/docs/stt/rt/real-time-transcription)
- [Soniox — WebSocket API](https://soniox.com/docs/api-reference/stt/websocket-api)
- [Soniox — Create transcription](https://soniox.com/docs/api-reference/stt/transcriptions/create_transcription)
- [Soniox — API reference index](https://soniox.com/docs/api-reference)
- [Symphonia docs.rs](https://docs.rs/symphonia)
- [Symphonia FAQ — iOS support](https://pdeljanov-symphonia.mintlify.app/resources/faq)

---

## Concrete next steps for FlowFlow

Ordered by impact / effort ratio:

1. **Add audio import to the document picker** (small, high value)
   - File: `src/platform/ios/picker.rs` — add an `audio` variant returning a path.
   - Add `audio` `AttachmentKind` (or reuse note `audio_file_path`) in `models/note.rs` / `models/attachment.rs`.
   - Route the picked path into the existing Soniox `transcribe` flow (it already handles WAV; m4a/mp3 will also work via Soniox auto-detect).
   - Register UTIs in `Info.plist` so the app shows up in AirDrop / Files share sheet.

2. **AVAudioSession** — **already optimal in FlowFlow** (`playAndRecord` + `DefaultToSpeaker | AllowBluetoothA2DP | MixWithOthers`). Optional refinement: drop `MixWithOthers` mid-recording (cosmetic, iOS ducks automatically).

3. **Skip call recording entirely** — document the limitation in the in-app help. Apple's native iOS 18+ Phone-app recorder is the only legal path and FlowFlow can't replicate it.

4. **System audio capture (Broadcast Upload Extension)** — defer to a "Phase 2" track. Estimated effort: 1–2 weeks for the Xcode extension + signing + App Group + Rust bridge to read the produced file. Add as future Track in CLAUDE.md. Limitations to surface to users: DRM-protected content (Music/Spotify/Netflix/Safari video) returns silence by design; only "free" application audio (e.g. games, podcast players using native AVAudio, third-party browsers using non-AVPlayer pipelines) gets captured.

5. **No Rust audio conversion needed** — keep things simple. If anything, consider switching the recorder backend from cpal+hound (`.wav`) to a thin Swift bridge using `AVAudioRecorder` writing m4a/AAC — 10× smaller uploads to Soniox, identical accuracy.
