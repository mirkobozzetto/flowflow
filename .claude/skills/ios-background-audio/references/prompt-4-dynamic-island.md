# Prompt 4 - Dynamic Island Live Activity

Self-contained task block. Estimated time: 1-2 days. Depends on BOTH Prompt 1 (background audio) and Prompt 3 (Dioxus upgrade).

## Preflight

Confirm Prompt 1 is complete: `background_modes = ["audio"]` must be present under `[ios]` in `Dioxus.toml`. Without it, the host app gets suspended in background and the Live Activity timer renders nothing meaningful.

Confirm Prompt 3 is complete:

```bash
bash .claude/skills/ios-background-audio/scripts/check-dx-version.sh
```

`dx --version` must report 0.7.9. If not, run Prompt 3 first.

Before running `make ddev` (physical device), mint the Widget Extension provisioning profile. The Widget Extension is a separate bundle with its own bundle id (`com.mirkobozzetto.flowflow.recording-widget`), so it needs its own profile. Apple's free-account workflow does not let `dx` create this profile automatically. Workaround:

1. In Xcode, File -> New -> Project -> iOS -> App.
2. Product Name: `flowflow-recording-widget`, Organization Identifier: `com.mirkobozzetto`, Team: Personal Team.
3. Open Signing & Capabilities, set bundle id to `com.mirkobozzetto.flowflow.recording-widget`. Cmd+R on a physical iPhone target.
4. Xcode mints the profile in `~/Library/Developer/Xcode/UserData/Provisioning Profiles/`. Trust the new dev profile on the iPhone (Settings -> General -> VPN & Device Management).
5. Delete the temporary Xcode project. The profile persists.

You now have two profiles: the host app one (already minted via the existing FlowFlow setup) plus the new widget one. Both expire after 7 days on a free Apple Developer account. Plan to re-mint weekly.

Run before any edit:

```bash
gitnexus_impact({target: "AudioRecorder", direction: "upstream"})
gitnexus_impact({target: "start", direction: "upstream"})
gitnexus_impact({target: "stop", direction: "upstream"})
```

Report blast radius for each. Warn on HIGH or CRITICAL.

## Block

```
<context>
FlowFlow is an iOS Rust/Dioxus 0.7.9+ app (after Prompt 3).
Dioxus 0.7.4+ supports Widget Extensions natively via Dioxus.toml.
PR #4842 ships ios.widget_extensions config, the Swift FFI pipeline, and bundling.

The user wants to see the recording timer in the Dynamic Island while the app records in background.

Dynamic Island uses ActivityKit + WidgetKit.
UI must be SwiftUI inside a Widget Extension (separate target).
4 layouts: compact leading, compact trailing, minimal, expanded.

Reference example: examples/01-app-demos/geolocation-native-plugin/ in the Dioxus repo.

Existing files:
- src/services/audio.rs: AudioRecorder with start/pause/resume/stop/cancel
  duration_secs() returns the current elapsed duration
- src/ui/recording/controls.rs: recording controls UI, 28-bar waveform, 60 ms refresh
- Dioxus.toml: iOS config
</context>

<task>
Implement the Dynamic Island for audio recording.

1. Dioxus.toml: add Widget Extension config
   [ios.plist]
   NSSupportsLiveActivities = true

   [[ios.widget_extensions]]
   source = "src/ios/widget"
   display_name = "FlowFlow Recording"
   bundle_id_suffix = "recording-widget"
   deployment_target = "16.2"
   module_name = "RecordingPlugin"

2. Lay out the Widget Extension as a SwiftPM package under src/ios/widget/:
   - src/ios/widget/Package.swift (SwiftPM manifest, platforms .iOS("16.2"), product library named RecordingWidget)
   - src/ios/widget/Sources/RecordingAttributes.swift (~30 lines)
       import ActivityKit
       struct RecordingAttributes: ActivityAttributes
       ContentState: startedAt (Date), isPaused (Bool)
   - src/ios/widget/Sources/RecordingWidget.swift (~150 lines)
       import WidgetKit, SwiftUI, ActivityKit
       4 layouts:
         Compact leading: red mic icon (circle fill)
         Compact trailing: timer MM:SS via Text(context.state.startedAt, style: .timer)
         Minimal: small red mic icon
         Expanded: large timer + icon + label "Recording in progress"
       Use .contentTransition(.numericText()) to animate the timer.

3. Lay out the host-side plugin (FFI bridge target) as a SwiftPM package under src/ios/plugin/:
   - src/ios/plugin/Package.swift (SwiftPM manifest, platforms .iOS("16.1"), product library named RecordingPlugin)
   - src/ios/plugin/Sources/RecordingPlugin.swift (the @_cdecl FFI entry points)
   - src/ios/plugin/Sources/RecordingAttributes.swift (DUPLICATE of the widget's RecordingAttributes.swift; the Widget Extension is a separate process and cannot link the plugin's Swift code at runtime)

4. Create the Rust FFI bridge src/platform/ios/live_activity.rs:
   - start_live_activity(started_at_unix: i64, is_paused: bool):
       calls flowflow_start_live_activity FFI which runs:
       Activity.request(
           attributes: RecordingAttributes(),
           content: ActivityContent(state: .init(startedAt: Date(timeIntervalSince1970: ...), isPaused: false), staleDate: nil),
           pushType: nil
       )
   - update_live_activity(started_at_unix: i64, is_paused: bool):
       activity.update(ActivityContent(state: ..., staleDate: nil))
   - end_live_activity():
       activity.end(nil, dismissalPolicy: .immediate)
   - Use raw objc2 bindings or @_cdecl Swift exports declared as extern "C" in Rust. manganis::ffi is experimental in Dioxus 0.7; do not depend on it.

5. Wire into AudioRecorder:
   - start() -> start_live_activity(now_unix, false)
   - pause() -> update_live_activity(started_at_unix, true)
   - resume() -> update_live_activity(now_unix, false) (reset startedAt so the widget timer restarts from the resume instant)
   - stop()/cancel() -> end_live_activity()
   - No 1 s push loop: the widget self-ticks via Text(startedAt, style: .timer).

6. make format && make check
7. Test on physical device (Dynamic Island requires iPhone 14 Pro+ or iPhone 15+)
</task>

<constraints>
- Widget Extension is Swift only (no Rust inside the widget)
- The Widget Extension and the plugin are each a SwiftPM package (Package.swift + Sources/). Not flat .swift files in the source dir.
- RecordingAttributes.swift MUST be duplicated in both widget and plugin Sources/ (separate processes, no runtime linking between them)
- The Rust FFI bridge uses raw objc2 or @_cdecl Swift exports. manganis::ffi is experimental and not stable in Dioxus 0.7.
- Zero comments in code (Rust and Swift)
- For the Dynamic Island timer, use Text(context.state.startedAt, style: .timer) so the widget self-refreshes. Do not push elapsedSeconds.
- Do not commit without explicit user approval
- Prerequisites: Prompt 1 (background_modes = ["audio"] in [ios]) AND Prompt 3 (Dioxus 0.7.9)
</constraints>

<success_criteria>
- Start recording -> Dynamic Island shows the timer
- Move to background -> Dynamic Island still visible with the timer
- Pause -> Dynamic Island shows pause state
- Stop -> Dynamic Island disappears
- Works on a physical iPhone 14 Pro+ or iPhone 15+
- make check is clean
- gitnexus_detect_changes() reports only expected scope
</success_criteria>
```

## Notes

- ActivityKit requires iOS 16.1+ for `Activity.request()`; Dynamic Island UI requires iOS 16.2+ on the widget. Set `deployment_target = "16.2"` on the widget extension; the host app stays at iOS 16.0.
- `NSSupportsLiveActivities = true` must live in the host app's Info.plist (`[ios.plist]`), not the widget's.
- The Widget Extension is a fully separate bundle with its own bundle id. It needs its own provisioning profile (see the Preflight section for the Xcode workaround on a free Apple Developer account).
- Update cadence: ActivityKit throttles to ~one push per second. Use the SwiftUI auto-refreshing `Text(context.state.startedAt, style: .timer)` so the widget ticks without any push from the app, and only call `update_live_activity` on pause/resume.
- `Activity.request()` is foreground only. FlowFlow always starts the activity from the foreground (the user taps the record button), so this is not a blocker.
- Swift 6: `Activity` is not `Sendable`. In the plugin, wrap calls in `Task { @MainActor in ... }` to satisfy strict concurrency.

## See also

- `references/activitykit-overview.md` for ActivityKit + WidgetKit primer and the Swift struct shape
- `references/dioxus-widget-config.md` for the `[[ios.widget_extensions]]` schema and the dx build pipeline
- `references/prompt-1-background-audio.md` for the background recording prerequisite
- `references/prompt-3-dioxus-upgrade.md` for the prerequisite toolchain bump
