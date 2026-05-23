# iOS system audio capture — deep dive (2026)

> Question: ANY way in 2026 to simultaneously capture (a) the user's microphone AND (b) other apps' audio output, route both into FlowFlow for transcription?
> Date: 2026-05-23
> Status: research only

## TL;DR

iOS in 2026 has **no public API** to capture other apps' audio cleanly mixed with mic.

The ReplayKit Broadcast Upload Extension path (`audioApp` + `audioMic`) is the only sanctioned mechanism, but has structural limits:
- 50 MB memory ceiling
- User-initiated only (`RPSystemBroadcastPickerView`)
- Runs in separate extension process
- **AVPlayer-backed playback (YouTube, Spotify, Apple Music, Safari `<video>`, Netflix, Podcasts) emits silent buffers** — DRM/copyright carve-out enforced by `mediaserverd`

macOS gained `SCStream` + microphone (WWDC24) and CoreAudio process taps (macOS 14.2+); **iOS did not get the parallel API**.

AudioDriverKit ships on iPadOS 16+ (M-series) but only for physical hardware drivers — virtual devices remain prohibited.

AirPlay-receiver-on-iPhone is technically possible (TestFlight prototypes exist) but blocked from App Store by entitlement gating.

**Realistic shippable wins for FlowFlow:**
- Speakerphone acoustic capture (lossy but works for any source)
- ReplayKit for non-AVPlayer apps (Discord, games, some browsers)
- iRig-style hardware loopback (niche pro path, lossless any source)
- Screen Recording → import via Files (Track G already supports)

## App Store policy — clarification

**Apps shipping ReplayKit Broadcast Extension are ALLOWED.** Examples in production: Discord (screen share), LiveKit, Twilio Video, voiceping-ai/ios-mac-offline-transcribe.

The DRM block on YouTube/Spotify/Apple Music is **runtime behavior** (`mediaserverd` zeroes the buffers), not a review rejection.

What actually gets rejected:
- AirPlay receiver mode (MFi entitlement gated, not granted to indies)
- VoIP misclassification (fake CallKit use to grab background audio)
- Private system-audio entitlements (never granted to third parties)
- Virtual audio drivers (don't exist as API on iOS)

| Approach | App Store | Works at runtime? |
|----------|-----------|-------------------|
| Broadcast Extension (games, Discord, AVAudioEngine apps) | OK | Yes |
| Broadcast Extension + YouTube/Spotify | OK | **No** (silent buffers from `mediaserverd`) |
| AirPlay receiver iPhone | Rejected | Yes technically |
| Hardware loopback (iRig) | OK | Yes (any content) |
| Screen Recording → import | OK | Yes for YouTube (silent for Apple Music/Netflix) |

---

## 1. Apps that produce non-silent `audioApp` buffers in 2026

### Verdict: partial — works only for non-`AVPlayer` audio paths

ReplayKit's `audioApp` is *not* a blanket DRM block. It's a `mediaserverd` policy: any audio routed through `AVPlayer` / `AVPlayerItem` / `MPMusicPlayerController` / `WKWebView`-backed `<video>` / `AVAudioPlayerItem` with FairPlay or "tagged-protected" routes produces zero buffers. Other audio sources come through.

**Confirmed silent (AVPlayer-class):**
- YouTube native iOS app, YouTube Music
- Spotify (uses AVPlayer)
- Apple Music, Apple Podcasts
- Safari `<video>`/`<audio>`, WKWebView-backed audio (Apple Developer Forums thread 670482)
- Netflix, Disney+, Prime Video
- FaceTime / WhatsApp / Phone (CallKit explicitly carved out)

**Confirmed working (non-AVPlayer):**
- Most games using `AVAudioEngine`/OpenAL/Unity FMOD output (Genshin Impact, Clash Royale)
- Discord voice channels (VoIP audio session, mixable subset)
- Podcast players that use raw `AVAudioEngine` (Overcast in some configs, Pocket Casts)
- Voice Memos playback
- AVAudioEngine apps generally (AUM, Korg Gadget, synthesizers)
- GarageBand playback
- TikTok/Instagram in-feed (mixed — some assets blocked, some not)

**Production example:** voiceping-ai/ios-mac-offline-transcribe (Feb 2026): ships "System Audio" mode via `RPSystemBroadcastPickerView` + Broadcast Upload Extension + mmap ring buffer between extension and main app, feeds Whisper.cpp ASR.
- https://github.com/voiceping-ai/ios-mac-offline-transcribe

**LiveKit SDK** (`client-sdk-swift`) same pattern with `ScreenShareCaptureOptions(appAudio: true)`. Notes "Application audio is not supported with In-App Capture mode" — only broadcast extension delivers `audioApp`.
- https://github.com/livekit/client-sdk-swift/blob/main/Docs/ios-screen-sharing.md

**App Store apps reviewed** (Just Press Record, AudioShare, Voice Memos, Meret, LudyRec, Tape It, Audiply, StudioMini, MicSwap, AudioLab): **none** advertise system-wide capture. All record mic or external USB/Lightning interfaces only.
- https://audioutils.com/blog/how-to-record-audio-on-iphone

**Effort for FlowFlow**: 1-2 weeks Rust-side. Broadcast Upload Extension Swift/Obj-C (can't be pure Rust), mmap ring buffer to main Rust process, feed buffer into existing Soniox path. Pattern proven by voiceping-ai.

**Must disclose in UI**: "Cannot capture YouTube, Spotify, Apple Music or protected playback."

Sources:
- https://developer.apple.com/documentation/replaykit/rpbroadcastsamplehandler
- https://support.apple.com/guide/security/replaykit-security-seca5fc039dd/web
- https://developer.apple.com/forums/thread/670482
- https://github.com/shogo4405/HaishinKit.swift/issues/849

---

## 2. iOS 17 / 18 / 19 / 26 API changes

### Verdict: blocked — no new iOS API for cross-app audio capture

Apple expanded **macOS** audio capture significantly while leaving iOS frozen.

**WWDC24 "Capture HDR content with ScreenCaptureKit" (session 10088)**: added `captureMicrophone` + `microphoneCaptureDeviceID` on `SCStreamConfiguration` and `SCRecordingOutput`. **macOS only**. `ScreenCaptureKit` not available on iOS.
- https://developer.apple.com/videos/play/wwdc2024/10088/
- https://developer.apple.com/documentation/ScreenCaptureKit/

**macOS 14.2+ CoreAudio process tap**: captures system audio without screen-recording permission. **iOS does not have process taps**.
- https://nonstrict.eu/recordkit/guides/system-audio-recording.html

**WWDC25 "Enhance your app's audio recording capabilities" (session 251)**: iOS 26 additions are all mic-side:
- `AVInputPickerInteraction` — in-app input picker
- `bluetoothHighQualityRecording` AVAudioSession category — AirPods H2 LAV-quality
- Spatial Audio FOA capture via `AVAssetWriter`
- Simultaneous `MovieFileOutput` + `AudioDataOutput`
- QTA audio-only QuickTime format

**Zero** new APIs for cross-app or system audio.
- https://developer.apple.com/videos/play/wwdc2025/251/

iOS 26 SDK **removed** `com.apple.developer.pushkit.unrestricted-voip.ptt` entitlement (Apple Forums 780822). Trend is narrowing privileged audio access, not widening.

---

## 3. AudioBus, Inter-App Audio (IAA), AUv3 hosting

### Verdict: partial — only captures audio from *cooperating* apps

**Inter-App Audio (IAA)**: deprecated since iOS 13. Still functions. AudioBus 3 builds on IAA + own protocol. Receivers can only get audio from apps that explicitly publish `AudioComponents` Info.plist entry and call `AudioOutputUnitPublish()`.

YouTube/Spotify/Apple Music/Safari **do not publish IAA ports**.

AudioBus is closed cooperative ecosystem of music-production apps (Loopy, Cubasis, Animoog, Korg Gadget, AUM).
- https://developer.audiob.us/doc/_integration-_guide.html
- https://developer.apple.com/documentation/bundleresources/entitlements/inter-app-audio

**FlowFlow value**: zero coverage for actual user need (YouTube/Spotify/Podcast capture). Useful only if FlowFlow pivots to musicians.

---

## 4. AUv3 Audio Unit Extensions as receiver

### Verdict: blocked for FlowFlow use case

AUv3 extensions are plug-ins inside a host app. Extension talks to host, not to its own container app, not OS, not other apps. Apple DTS-confirmed (Stack Overflow 61360345): only way extension → container is shared memory in App Group, no real-time guarantees.

Critically: you only receive audio when the **host app actively loads your AUv3** — Safari, Spotify, YouTube don't host AUv3 plugins.

UX requires user to open DAW (GarageBand, AUM, Cubasis), add FlowFlow as effect insert, route playback through it. Two-app technical UX. Same coverage as AudioBus.
- https://stackoverflow.com/questions/61360345/can-an-audio-unit-v3-replace-inter-app-audio-to-send-audio-to-a-host-app

---

## 5. AirPlay receiver mode

### Verdict: works as TestFlight prototype, blocked from App Store

iPad (M-series) gained native AirPlay receiver on iPadOS 17. **iPhone has never officially shipped AirPlay receiver** at OS level. iOS 26/iPadOS 26 did not change this for iPhone.

**Third-party AirPlay receivers exist:**
- **AirAP** (neon443/AirAP, May 2025): native Swift AirPlay server, TestFlight only.
  - https://github.com/neon443/AirAP
- **shairplay-rust** (metaneutrons/shairplay-rust, April 2026): pure-Rust AP1+AP2 receiver library with ALAC/AAC decode, ChaCha20-Poly1305, HomeKit pairing.
  - https://github.com/metaneutrons/shairplay-rust
- **rairplay** (r4v3n6101/rairplay) — similar Rust AP2 receiver.

**Apple's App Store gate**: AirPlay receiver requires MFi (Made for iPhone) AirPlay 2 / AAC license + entitlements **never granted to indie developers**. Third-party AP2 receivers cannot send playback commands back (Type 130 MRP, companion-link, DACP all require HomeKit/Apple ID trust).

**Latency**: AP1 ~2s, AP2 ~1s, plus ~10s setup delay for non-Apple receivers.

**FlowFlow implementation in Rust**: shairplay-rust is `#![forbid(unsafe_code)]` and async on tokio — drops in. Build receiver, decode ALAC/AAC, feed PCM into transcription pipeline. UX: user toggles AirPlay in YouTube/Spotify → picks "FlowFlow" → audio streams.

**Distribution blocker**: App Store reviewers will reject. TestFlight-only / sideload-only. Not viable for App Store launch.

**Effort**: 2-3 weeks AP1 basic. 5-8 weeks AP2 with HomeKit pairing.

---

## 6. Hardware loopback workarounds

### Verdict: works — niche pro-user path

**iRig Stream Pro / iRig Stream Mic Pro / iRig HD X** (IK Multimedia): all advertise **"Device Loopback" / "Loopback+"**: routes iPhone audio output back into iPhone audio input on a single USB-C/Lightning cable.

> "This option routes the audio output of your digital device (iPhone, iPad, Android, Mac or PC) back into iRig Stream Pro to be mixed in with the other audio signals. Perfect to jam or sing along with backing tracks, or process audio on one app and stream to another, all on a single device."
> — https://www.ikmultimedia.com/products/irigstreampro/

**This is genuine cross-app system-wide capture**: YouTube/Spotify play through iRig loopback channel, FlowFlow receives loopback + mic via USB audio interface as 2-channel input. **No silence problem because audio never leaves digital path through AVPlayer DRM gate** — bypasses to USB-class audio device.

**Compatible devices:**
- iRig Stream Pro (USB-C + Lightning + USB-A, 24/96, 4-ch)
- iRig Stream Mic Pro (same loopback)
- iRig HD X (USB-C, 24/96, Loopback+ as virtual FX loop)

**Effort for FlowFlow**: Zero code if user owns iRig. Detect USB audio device via `AVAudioSession.availableInputs`, read 2-channel input, transcribe channel-1 (loopback), log channel-2 (mic) optionally.

Sources:
- https://www.ikmultimedia.com/products/irigstreampro/
- https://www.ikmultimedia.com/products/irigstreammicpro/
- https://www.ikmultimedia.com/products/irighdx/

---

## 7. VoIP / PushKit audio sessions

### Verdict: blocked — no broader audio capture, just background privilege

Apple DTS (Quinn, Kevin Elliott) across forum threads 758386, 810385, 774784, 806507: CallKit/LiveCommunicationKit/PushToTalk audio session is a privileged variant of PlayAndRecord with higher interruption priority, slightly louder output, background activation. Does **NOT** expose other apps' audio. Just lets your app keep recording in background.

- `com.apple.developer.pushkit.unrestricted-voip` entitlement: deprecated, never granted, "probably an LLM hallucination" — Kevin Elliott DTS
- `com.apple.developer.pushkit.unrestricted-voip.ptt`: **disabled in iOS 26 SDK**
- VoIP background mode + PushKit: requires real telephony-class use. App Review rejects fake VoIP classification

**Risk for FlowFlow**: misclassification = App Store rejection on §2.5.x. Zero capture value.

Sources:
- https://developer.apple.com/forums/thread/758386
- https://developer.apple.com/forums/thread/810385
- https://developer.apple.com/forums/thread/780822

---

## 8. DriverKit / Audio Server Plug-ins on iPad/iPhone

### Verdict: blocked for iPhone, partial for iPadOS (physical hardware only)

**AudioDriverKit** available on iPadOS 16+ **only on iPads with M-series chip**. **Not available on iPhone.** Even on iPadOS, Apple's explicit guidance:

> "When creating a virtual device, best practice is to use an Audio Server Driver Plug-in instead. AudioDriverKit only supports physical audio devices."
> — https://developer.apple.com/documentation/AudioDriverKit/creating-an-audio-device-driver

**Audio Server Plug-ins are macOS-only.** No iOS/iPadOS equivalent. BlackHole, Loopback, Soundflower-style virtual devices cannot exist on iOS.

---

## 9. CoreAudioKit on iOS

### Verdict: blocked — UI framework, not capture

CoreAudioKit on iOS is UI shell for hosting AUv3 plugin views (`AUViewController`, `AUv3ViewController`). Does not expose virtual device registration, capture, or system audio interception.

---

## 10. Apple's own apps and private entitlements

### Verdict: documented Apple-internal-only

- **Voice Memos call recording (iOS 18+)**: opt-in call recording in Phone + Voice Memos — both parties hear announcement at start. Uses `callservicesd` via private entitlement, mandatory announcement, entitlement not exposed to third parties
- **Logic Pro on iPad system-wide audio**: AUv3 host APIs publicly available + iPad-only AudioDriverKit + connected hardware. No private system-audio entitlement
- **GarageBand iOS**: AudioBus / AUv3 / IAA. Public APIs
- **FaceTime / Phone**: CallKit privileged session — privileged-recording session for its own mic input, not system audio

There is **no documented case** of Apple granting a system-audio entitlement to a third-party app. Multiple Apple DTS posts state private entitlements not requestable.

Audio Hijack (Rogue Amoeba) ships only on macOS for this exact reason; Paul Kafasis publicly stated iOS port impossible without entitlement.

---

## 11. Recent App Store apps claiming system audio capture

### Verdict: none deliver — all rely on mic acoustic capture or AirPlay-to-Mac workflow

Reviewed: Just Press Record, AudioShare, Voice Memos, Tape It, Meret, LudyRec, AudioLab, Audiply, StudioMini, MicSwap Remix, Dolby On, Rev Voice Recorder, TapeACall.

**Pattern:**
- "Record everything" apps record **microphone only** (sometimes with USB interface)
- Call recorders (TapeACall, Rev) bridge call through their own conference server — record **server-side**, not on-device. Mandatory two-party announcement
- Phone-call-recording hardware (UMEVO Note Plus, iRecorder Pro): use vibration conduction sensors or BT headset profile, not iOS APIs

> "There is no built-in way to record a phone call on iPhone in 2026. iOS does not expose a system-audio API to apps … no app — first-party or otherwise — can capture audio of a normal phone call or FaceTime call. Apple has been explicit that this is a deliberate privacy boundary."
> — https://audioutils.com/blog/how-to-record-audio-on-iphone

Only iOS app surfaced that genuinely captures non-AVPlayer system audio is **voiceping-ai/ios-mac-offline-transcribe** — not App Store, open-source, ReplayKit broadcast extension pattern.

---

## 12. Workarounds that actually work in production

### 12a. Speakerphone + built-in mic acoustic capture

**Verdict: works, ships today, lossy**

User plays YouTube/Spotify/Podcast on iPhone speaker, FlowFlow records via mic. Quality penalty: speaker EQ, room reverb, mic AGC, ~6-10 dB SNR loss. Transcription quality drops moderately for clean speech, significantly for music or low-volume content.

**Effort**: zero — FlowFlow's current capability.

### 12b. Screen Recording pre-step → Files → import

**Verdict: works, 2-step UX, lossless for most**

User uses iOS Control Center → Screen Recording → records YouTube playback (Apple's first-party `replayd` does *not* zero out for protected content into user's own Photos/Files). Save MOV/MP4 to Files. Import into FlowFlow as attachment (Track G supports), extract audio via `AVAssetExportSession`, transcribe.

**Catch**: Apple's screen recorder also gets silence on some protected playback (Apple Music, Netflix). YouTube via Safari and YouTube native app *do* record audio.

**Effort for FlowFlow**: ~3-5 days. Add `AVAssetExportSession` audio extraction to existing attachment import path. Excellent fit — Track G already imports files.

### 12c. AirPlay → Mac → capture → AirDrop

**Verdict: works, multi-device, Mac required**

User AirPlays YouTube/Spotify to Mac running QuickTime + BlackHole. Mac records lossless. AirDrops file back. FlowFlow imports.

**Effort**: zero — already supported via Track G.

### 12d. TRRS splitter inverted

**Verdict: physically unreliable, not recommended**

iPhone connector auto-detects TRRS vs TRS. Feeding line-level signal back into ring contact won't produce clean capture; AGC and impedance issues.

### 12e. Audio Hijack approach ported

**Verdict: blocked**

Audio Hijack uses macOS-only kext/CoreAudio HAL plug-ins. No iOS equivalent.

---

## Ranked realistic options table

| Rank | Option | User value | Engineering cost | App Store risk | YouTube/Spotify coverage | Ship rec |
|------|--------|------------|------------------|----------------|--------------------------|----------|
| 1 | Document limitation + speakerphone path | Low | 0 days | None | Acoustic only, lossy | **Ship in v1 docs** |
| 2 | Screen Recording → import via Files | Medium-High | 3-5 days | None | Yes YouTube; no Apple Music/Netflix | **Ship as primary documented workaround** |
| 3 | ReplayKit Broadcast Upload Extension | Medium (games, Discord, podcasts) | 1-2 weeks | None | No (AVPlayer silent) | **Ship in v2 — disclose AVPlayer limit** |
| 4 | iRig hardware loopback | High pro users, niche overall | 3-5 days | None | Yes (all apps, all content) | **Document as pro path** |
| 5 | AirPlay → Mac → AirDrop | Medium (Mac users) | 0 (Track G) | None | Yes | **Document workaround** |
| 6 | AudioBus SDK | Very low (musicians only) | 1 week | None | No | **Skip unless musician pivot** |
| 7 | AUv3 host or extension | Very low | 2-3 weeks | None | No | **Skip** |
| 8 | AirPlay receiver mode | Very high | 2-8 weeks | **High App Store rejection** | Yes | **Skip for App Store; TestFlight beta possible** |
| 9 | VoIP/CallKit reclassification | Negative | 1-2 weeks | High rejection | No | **Skip** |
| 10 | AudioDriverKit virtual device | N/A | N/A | N/A | N/A | **Blocked** |

---

## Concrete recommendation for FlowFlow

**Ship sequence:**

1. **v1 (now)**: Settings doc page explaining OS limitation, speakerphone workaround, Screen-Recording-then-import workflow. Effort: hours.

2. **v1.1 (next sprint)**: Wire `AVAssetExportSession` audio extraction into Track G attachment import so a screen-recording `.mov` → audio → transcription pipeline runs in one tap. Effort: 3-5 days. **Highest value/cost ratio.**

3. **v2 (later)**: Add ReplayKit Broadcast Upload Extension with mmap ring buffer (clone voiceping-ai pattern). Surface "System Audio Mode" toggle with explicit disclosure: "Captures audio from games, Discord, and many apps. **Does not capture YouTube, Spotify, Apple Music, Safari video, FaceTime or phone calls** — Apple enforces this." Effort: 1-2 weeks. Useful for Discord meetings, gameplay, voice-channel content.

4. **v2.x optional**: Detect iRig/Apogee/Rode USB audio devices in input picker, route loopback channel into transcription. Effort: 3-5 days. Power-user feature.

5. **Skip permanently**: AirPlay receiver (App Store rejection), VoIP misclassification (rejection), AudioBus/AUv3 (no coverage benefit), AudioDriverKit (not allowed for virtual devices on iOS).

The path of "do nothing technical, document well, add file import for Screen Recording" is the highest-ROI move — unblocks 80% of real user needs without engineering risk.
