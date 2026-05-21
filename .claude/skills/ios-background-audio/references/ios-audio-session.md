# AVAudioSession Reference

Quick reference for AVAudioSession configuration as used by FlowFlow. Source: Apple Developer Documentation, `AVFAudio` framework.

## Category

| Category | Behaviour |
|----------|-----------|
| `PlayAndRecord` | Mic input + speaker output. Required for FlowFlow recording UX. Allows mixing if `MixWithOthers` is set. |
| `Record` | Mic input only. No output. Mutes other audio. |
| `Playback` | Output only. Cannot record. |
| `Ambient` | Output only, mixes with other apps, silenced by ring/silent switch. |

FlowFlow uses `PlayAndRecord`. Do not change without explicit user approval.

## Category Options (bitmask)

| Option | Effect |
|--------|--------|
| `DefaultToSpeaker` | Route output to the loudspeaker instead of the earpiece. |
| `AllowBluetoothA2DP` | Allow A2DP Bluetooth output (headphones, speakers). |
| `MixWithOthers` | Do not interrupt audio from other apps when activating this session. Required to coexist with YouTube/Music/Podcasts. |
| `AllowBluetoothHFP` | Allow Hands-Free Profile Bluetooth (mic + output, lower fidelity). |
| `DuckOthers` | Reduce volume of other apps while this session is active. |

FlowFlow target: `DefaultToSpeaker | AllowBluetoothA2DP | MixWithOthers`.

`MixWithOthers` is required for background reliability. Without it, the session can fail to activate after returning from background and surface error `560557684` (`AVAudioSessionErrorCodeCannotStartRecording`). Reference: <https://github.com/llfbandit/record/issues/542>.

## cpal on iOS

`cpal` 0.17 on iOS targets uses CoreAudio RemoteIO under the hood via `objc2-avf-audio`. The cpal `Stream` is `!Send`, so it cannot be moved into a notification block. Use an `mpsc` channel (or `Arc<Mutex<...>>`) to marshal interruption events from the Objective-C callback into the thread that owns the stream.

## Activation

Activate via `setActive(true)`. Deactivate via `setActive(false)`. Rules:

- Activate before starting a `cpal` stream.
- NEVER deactivate while a recording is active. Deactivating tears down the audio route and CoreAudio loses the input device.
- It is safe to keep the session active across recording pauses; only stop the cpal stream.
- On interruption Began, do NOT call `setActive(false)`. iOS auto-suspends the route; calling `setActive(false)` makes the resume path noisy.

## Interruption Notification

Notification name: `AVAudioSessionInterruptionNotification`.

Userinfo keys (objc2 names):

| Key | Type | Meaning |
|-----|------|---------|
| `AVAudioSessionInterruptionTypeKey` | `UInt` (enum) | `1` = Began, `0` = Ended |
| `AVAudioSessionInterruptionOptionKey` | `UInt` (bitmask) | Present on Ended only |
| `AVAudioSessionInterruptionWasSuspendedKey` | `Bool` | True when the interruption was caused by the system suspending the app |
| `AVAudioSessionInterruptionReasonKey` | `UInt` (iOS 14.5+) | Reason code (call, route change, etc.) |

`AVAudioSessionInterruptionOptions` bitmask:

| Bit | Constant | Meaning |
|-----|----------|---------|
| 1 | `ShouldResume` | iOS thinks the app should resume audio. Read this and decide. |

## Observing the Notification (objc2)

Use the block-based observer API on `NSNotificationCenter`:

```rust
NSNotificationCenter::defaultCenter()
    .addObserverForName_object_queue_usingBlock(name, None, None, block);
```

Required cargo features:

- `objc2-foundation`: `NSNotification`, `block2`, `NSOperation`
- `objc2-avf-audio`: `0.3`, `AVAudioSession`

The block runs off the main thread. Do not touch the `cpal` stream from inside the block. Forward `(InterruptionType, ShouldResume)` over an `mpsc::Sender` and process it on the audio thread.

## Recommended Interruption Flow

1. `InterruptionType = Began`:
   - cpal stream is already stopped by the OS (RemoteIO suspended).
   - Set `RecordingState::Paused`.
   - Do NOT call `setActive(false)`.

2. `InterruptionType = Ended`:
   - If `Options & ShouldResume`: call `setActive(true)`, rebuild the cpal stream, set `RecordingState::Recording`.
   - Otherwise: keep `Paused`. The user resumes manually.

## Background Audio

Add `background_modes = ["audio"]` under `[ios]` in `Dioxus.toml` (PR #4842 schema, shipped in Dioxus 0.7.4+). `dx` emits the matching `UIBackgroundModes = ["audio"]` into the generated Info.plist. Without it, iOS suspends the app after ~5 seconds in background and `cpal` stops receiving samples. With it, the OS keeps the process running as long as the audio session stays active.

## References

- Apple docs: <https://developer.apple.com/documentation/avfaudio/avaudiosession>
- objc2 bindings: `objc2-avf-audio` crate, version 0.3
- Interruption notification keys: <https://developer.apple.com/documentation/avfaudio/avaudiosession/interruptionnotification>
- llfbandit/record bug #542 (MixWithOthers fix): <https://github.com/llfbandit/record/issues/542>
