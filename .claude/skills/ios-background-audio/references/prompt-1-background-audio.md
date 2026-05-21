# Prompt 1 - Background Audio Recording

Self-contained task block. Estimated time: 10 minutes. Standalone (no prerequisites).

## Preflight

Run before any edit:

```bash
bash .claude/skills/ios-background-audio/scripts/grep-setactive-calls.sh
```

Report every match. If any call passes `false` while a recording is active, stop and flag it. Run `gitnexus_impact({target: "configure_audio_session", direction: "upstream"})` and report the blast radius.

## Block

```
<context>
FlowFlow is a 100% Rust iOS app, Dioxus 0.7, cpal 0.17.
Recording stops when the app moves to the background.
Cause: UIBackgroundModes is absent from Dioxus.toml.
cpal uses CoreAudio RemoteIO, which respects AVAudioSession.
With UIBackgroundModes audio declared and the session active, the stream continues in background.

Files:
- Dioxus.toml: iOS configuration lives under [ios] (PR #4842 schema). Use background_modes here, not UIBackgroundModes under [ios.plist].
- src/platform/ios/mod.rs: configure_audio_session() uses AVAudioSession
  Current options: DefaultToSpeaker | AllowBluetoothA2DP
  Category: PlayAndRecord
</context>

<task>
Enable iOS background audio recording.

1. In Dioxus.toml under [ios], add:
   background_modes = ["audio"]
   This is the canonical PR #4842 schema (verified from the geolocation-native-plugin example). dx maps this to UIBackgroundModes in the generated Info.plist.

2. In src/platform/ios/mod.rs::configure_audio_session(), add MixWithOthers to the options:
   options = DefaultToSpeaker | AllowBluetoothA2DP | MixWithOthers

3. Verify no setActive(false) call fires during an active recording.
   Search every setActive call site under src/ and audit the conditions.

4. make format && make check
</task>

<constraints>
- Zero comments in code (Rust and Swift)
- Do not modify the RecordingState state machine
- Do not modify the cpal stream
- System audio capture is impossible on iOS (mic only), do not try
- Do not commit without explicit user approval
</constraints>

<success_criteria>
- Record then switch to Gmail then return: recording continued
- Record while YouTube plays: both work together
- make check is clean
- gitnexus_detect_changes() reports only expected scope
</success_criteria>
```

## Notes

- `MixWithOthers` is what lets FlowFlow record while another app plays audio. Without it, AVAudioSession evicts other audio sessions on activation.
- `background_modes = ["audio"]` in `[ios]` is the single switch that keeps the process running while recording. dx emits `UIBackgroundModes = ["audio"]` in the generated Info.plist. iOS will terminate the app in background within ~5 seconds otherwise.
- Use the `[ios]` table, NOT `[ios.plist]`. Setting `UIBackgroundModes` directly under `[ios.plist]` is the wrong schema and will not be picked up by the Dioxus 0.7.4+ build pipeline.

## See also

- `references/ios-audio-session.md` for category/option semantics
- `scripts/grep-setactive-calls.sh` for the preflight scan
