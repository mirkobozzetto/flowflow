---
name: ios-background-audio
description: Execute one of 4 iOS audio tasks for FlowFlow (Rust/Dioxus 0.7), (1) enable background audio recording via background_modes + MixWithOthers, (2) handle AVAudioSession interruptions (phone calls/FaceTime) with auto-resume, (3) upgrade Dioxus 0.7 to 0.7.9, (4) implement Dynamic Island Live Activity for recording timer via ActivityKit. Use when user mentions "background audio", "AVAudioSession", "interruption handling", "Dynamic Island", "Live Activity", "Dioxus upgrade", "UIBackgroundModes", "cpal background", "Widget Extension", or asks to run a numbered prompt (1-4) from the background-audio plan.
argument-hint: <1|2|3|4> (1=bg audio, 2=interruptions, 3=Dioxus upgrade, 4=Dynamic Island)
allowed-tools: [Read, Edit, Write, Bash, Glob, Grep]
---

# iOS Background Audio + Dynamic Island

Execute one of 4 self-contained iOS audio tasks for FlowFlow. Each task has a dedicated prompt reference with full context, success criteria, and constraints.

## Overview

FlowFlow is a 100% Rust iOS app (Dioxus 0.7, cpal 0.17) for voice recording with transcription and RAG. The 4 tasks unblock production-grade background recording and surface the recording timer in the iOS Dynamic Island.

Tasks:

1. Background audio recording (`[ios] background_modes = ["audio"]` + MixWithOthers)
2. AVAudioSession interruption handling (calls/FaceTime auto-resume)
3. Dioxus upgrade 0.7.0 to 0.7.9 (enables widget extensions)
4. Dynamic Island Live Activity (recording timer via ActivityKit)

## Argument Routing

This skill takes one argument `<N>` where N is 1, 2, 3, or 4.

Routing logic:

- If `$ARGUMENTS` empty or invalid: print the menu below and stop. Do not assume a task.
- If `$ARGUMENTS` is `1`: load `references/prompt-1-background-audio.md` and execute its task block.
- If `$ARGUMENTS` is `2`: confirm task 1 is completed (`background_modes = ["audio"]` present under `[ios]` in `Dioxus.toml`) before loading `references/prompt-2-interruptions.md`. If not, print the prerequisite warning and stop.
- If `$ARGUMENTS` is `3`: load `references/prompt-3-dioxus-upgrade.md` and execute its task block.
- If `$ARGUMENTS` is `4`: confirm BOTH task 1 (`background_modes = ["audio"]` under `[ios]`) AND task 3 (`dx --version` reports 0.7.9) are completed before loading `references/prompt-4-dynamic-island.md`. If either prerequisite is missing, print the prerequisite warning and stop.

Menu (printed when argument missing):

```
ios-background-audio: pick a task
  1 - Background audio recording
  2 - Interruption handling (requires 1)
  3 - Dioxus 0.7.9 upgrade
  4 - Dynamic Island Live Activity (requires 1 and 3)
Usage: /ios-background-audio <1|2|3|4>
```

Before executing the task block from the chosen prompt file, run the relevant safety script in `scripts/` if listed in the prompt's preflight section.

## Shared Constraints

These constraints apply to every task. Re-read them before each edit.

- Zero comments in any code produced (Rust and Swift).
- No emojis in code, commit messages, or PR descriptions.
- Never commit, push, or open a PR without explicit user approval. Default is no.
- Always run `make format && make check` after editing Rust source. The user runs `make build`, `make dev`, and `make ddev`.
- Before editing any symbol referenced in the prompts (functions, structs, services), run `gitnexus_impact({target: "name", direction: "upstream"})` and report blast radius. Run `gitnexus_detect_changes()` before claiming the task complete.
- Capture of system audio (YouTube, calls) is blocked by Apple on iOS. Mic only. Do not try to bypass.
- Never call `setActive(false)` on AVAudioSession while a recording is active.
- Do not modify the `cpal` stream or the `RecordingState` enum's public shape.

## Task Summaries

### Task 1 - Background audio recording (10 min, standalone)

Add `background_modes = ["audio"]` to `Dioxus.toml` under `[ios]` (canonical PR #4842 schema, NOT `UIBackgroundModes` under `[ios.plist]`) and add `MixWithOthers` to the AVAudioSession options in `src/platform/ios/mod.rs` (`configure_audio_session()`). Verify no `setActive(false)` call happens during active recording. Full block: `references/prompt-1-background-audio.md`.

### Task 2 - Interruption handling (~1 h, depends on 1)

Observe `AVAudioSessionInterruptionNotification` via `NSNotificationCenter` in `src/platform/ios/mod.rs`. Add `on_interruption_began()` and `on_interruption_ended(should_resume)` to `AudioRecorder` in `src/services/audio.rs`. Bridge callbacks via `std::sync::mpsc::Sender<InterruptionEvent>` (cpal `Stream` is `!Send`, so the audio thread that owns the stream is the one that must process events). UI `recording_state` signal must flip Recording <-> Paused. Full block: `references/prompt-2-interruptions.md`.

### Task 3 - Dioxus upgrade to 0.7.9 (30 min, standalone)

Run `cargo update`, `dx self-update` (or `cargo install dioxus-cli@0.7.9 --force`). Verify `dx --version` is 0.7.9 and `futures 0.3.32` is resolved. `make build` then `make check`. Full block: `references/prompt-3-dioxus-upgrade.md`.

### Task 4 - Dynamic Island Live Activity (1-2 days, depends on 1 and 3)

Declare Widget Extension in `Dioxus.toml` (`[[ios.widget_extensions]]`). Lay out the widget and the plugin as SwiftPM packages: `src/ios/widget/{Package.swift, Sources/RecordingAttributes.swift, Sources/RecordingWidget.swift}` (4 layouts: compact leading/trailing, minimal, expanded) and `src/ios/plugin/{Package.swift, Sources/RecordingPlugin.swift, Sources/RecordingAttributes.swift}` (the attributes file is duplicated, not shared). Build Rust FFI bridge `src/platform/ios/live_activity.rs` with `start/update/end_live_activity` calls. Wire into `AudioRecorder` lifecycle. Full block: `references/prompt-4-dynamic-island.md`.

## Prerequisites Graph

| Task | Depends on | Reason |
|------|------------|--------|
| 1 Background audio | none | Pure config + options change |
| 2 Interruptions | 1 | Without `background_modes = ["audio"]`, stream dies before interruption notifications fire |
| 3 Dioxus upgrade | none | Toolchain bump, no app code dependency |
| 4 Dynamic Island | 1 and 3 | Without 1 the activity is useless (no background recording for the timer to track). Without 3 the Widget Extension pipeline (PR #4842) is not available. |

Do not start a dependent task until its prerequisite passes the success criteria and `make check` is clean.

## References Index

| File | Use for |
|------|---------|
| `references/prompt-1-background-audio.md` | Full prompt block for task 1 |
| `references/prompt-2-interruptions.md` | Full prompt block for task 2 |
| `references/prompt-3-dioxus-upgrade.md` | Full prompt block for task 3 |
| `references/prompt-4-dynamic-island.md` | Full prompt block for task 4 |
| `references/ios-audio-session.md` | AVAudioSession categories, options, interruption notification API reference |
| `references/activitykit-overview.md` | ActivityKit + WidgetKit primer, Swift structs, Dioxus FFI bridge contract |
| `references/dioxus-widget-config.md` | `Dioxus.toml` `[[ios.widget_extensions]]` schema from PR #4842 |
| `scripts/check-dx-version.sh` | Verify installed `dx` version, used before task 3 success check and task 4 preflight |
| `scripts/grep-setactive-calls.sh` | List every `setActive(...)` call site under `src/` for task 1 safety check |

## Source

Original research and prompt text: `docs/rnd/background-audio-dynamic-island.md` (FlowFlow repo, 2026-05-21).
