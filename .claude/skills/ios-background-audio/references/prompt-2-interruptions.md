# Prompt 2 - Interruption Handling

Self-contained task block. Estimated time: ~1 hour. Depends on Prompt 1.

## Preflight

Confirm Prompt 1 is complete: `background_modes = ["audio"]` must be present under `[ios]` in `Dioxus.toml` and `MixWithOthers` must be in the AVAudioSession options. If not, run Prompt 1 first.

Run before any edit:

```bash
gitnexus_impact({target: "AudioRecorder", direction: "upstream"})
gitnexus_impact({target: "RecordingState", direction: "upstream"})
gitnexus_impact({target: "configure_audio_session", direction: "upstream"})
```

Report blast radius for each. Stop and warn the user if any returns HIGH or CRITICAL.

## Block

```
<context>
FlowFlow is an iOS Rust/Dioxus app.
Audio recording now runs in background (Dioxus.toml [ios] background_modes = ["audio"]).
When a phone call arrives, iOS interrupts the audio session.
Without a handler, the recording is lost.

Files:
- src/platform/ios/mod.rs: AVAudioSession config, objc2 bindings
- src/services/audio.rs: AudioRecorder, RecordingState enum
  States: Idle, Recording, Paused, Transcribing, Transcribed(String), Error(String)
- src/ui/recording/controls.rs: recording UI controls
- src/ui/state.rs: AppState, recording_state: Signal<RecordingState>

Dependencies: objc2-avf-audio 0.3, objc2-foundation
</context>

<task>
Handle iOS audio interruptions (phone calls, FaceTime).

1. In src/platform/ios/mod.rs, observe AVAudioSessionInterruptionNotification:
   - Use NSNotificationCenter::defaultCenter().addObserver
   - On InterruptionTypeBegan: expose a callback/signal to Rust
   - On InterruptionTypeEnded: read the shouldResume flag, expose a callback

2. In src/services/audio.rs:
   - Add on_interruption_began(): pause the cpal stream, keep the session active
   - Add on_interruption_ended(should_resume: bool):
     If should_resume, reactivate the session and rebuild the cpal stream
     Otherwise stay paused

3. Wire iOS callbacks to AudioRecorder methods.
   Use std::sync::mpsc::Sender<InterruptionEvent> exclusively. Define InterruptionEvent as { Began, Ended { should_resume: bool } } in src/services/audio.rs. The objc2 block holds the Sender; the audio thread that owns the cpal Stream holds the Receiver and processes events synchronously between samples or via a poll loop.
   Do not wrap AudioRecorder in Arc<Mutex<...>>: cpal Stream is !Send, so a wrapper that crosses the objc2 block boundary will not compile.

4. UI recording_state signal must mirror the interruption:
   Recording -> Paused on began
   Paused -> Recording on ended + shouldResume

5. make format && make check
</task>

<constraints>
- Zero comments in code (Rust and Swift)
- Do not change the public shape of RecordingState
- Auto-resume only when shouldResume is true
- If shouldResume is false: leave the UI in Paused so the user can resume manually
- Do not commit without explicit user approval
</constraints>

<success_criteria>
- Record, receive a call, hang up: recording resumes automatically
- Record, FaceTime invite, decline: recording resumes automatically
- shouldResume false: UI shows Paused, user can resume manually
- make check is clean
- gitnexus_detect_changes() reports only expected scope
</success_criteria>
```

## Notes

- The interruption notification fires off the main thread. Marshal back to the audio thread via `std::sync::mpsc::Sender<InterruptionEvent>`. Do not use `Arc<Mutex<AudioRecorder>>`: cpal `Stream` is `!Send`, so `AudioRecorder` cannot be shared across threads. The mpsc channel keeps the Stream pinned to its owning thread and only sends a copy-on-the-wire enum across the boundary.
- Do not use `tokio::sync::mpsc` here. The objc2 block is synchronous Objective-C land, not an async runtime. `std::sync::mpsc` is the right primitive.
- Do not block the notification callback. `Sender::send` on a normal channel is non-blocking; do not call into the AudioRecorder directly from the block.
- `shouldResume` is delivered as `AVAudioSessionInterruptionOptions.shouldResume`. Read it with `objc2_avf_audio::AVAudioSessionInterruptionOptions::ShouldResume` mask.
- During `pause`, do NOT call `setActive(false)`. Keep the session active so the route stays primed; only stop the cpal stream.

## See also

- `references/ios-audio-session.md` for interruption notification keys and option bitmask values
- `references/prompt-1-background-audio.md` for the prerequisite session config
