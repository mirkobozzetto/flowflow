# ActivityKit + WidgetKit Overview

Primer for Live Activities and Dynamic Island integration on iOS 16.1+ (Dynamic Island UI requires 16.2 widget target). Source: Apple Developer Documentation, `ActivityKit` and `WidgetKit` frameworks.

## What is a Live Activity

A Live Activity is a small, real-time UI surface owned by your app but rendered by the system, visible on the Lock Screen and in the Dynamic Island. The UI is SwiftUI and lives in a separate Widget Extension target (separate process). The host app starts/updates/ends the activity; the system renders.

## Required Targets

- Host app: declares `NSSupportsLiveActivities = true` in its Info.plist. iOS 16.1+ for `Activity.request()`.
- Widget Extension: separate bundle, SwiftUI views, links against `WidgetKit` and `ActivityKit`. Deployment target 16.2+ for Dynamic Island layouts.

The `ActivityAttributes` struct must be visible in BOTH targets. The Widget Extension is a separate process and cannot link the host app's Swift code at runtime. Duplicate the file (or include it via a shared Swift package). For FlowFlow, the simplest path is to keep the same `RecordingAttributes.swift` source file in both `src/ios/plugin/` (host bridge) and `src/ios/widget/` (extension).

## Activity Lifecycle

```swift
let attributes = RecordingAttributes()
let content = ActivityContent(state: .init(elapsedSeconds: 0, isPaused: false), staleDate: nil)
let activity = try Activity.request(attributes: attributes, content: content, pushType: nil)
try await activity.update(ActivityContent(state: newState, staleDate: nil))
await activity.end(nil, dismissalPolicy: .immediate)
```

`request` returns an `Activity` handle. Hold it in a singleton so `update` and `end` can find it later. `request` is foreground only: FlowFlow always starts recording from the foreground, so this is fine.

`staleDate: nil` means the system uses the default 8-hour lifetime cap. Past that, the activity auto-ends.

## Attributes vs Content State

```swift
struct RecordingAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        var startedAt: Date
        var isPaused: Bool
    }
}
```

- `ActivityAttributes` is the immutable shape of the activity. Decide once at `request` time.
- `ContentState` is the mutable part. Every `update` pushes a fresh `ContentState`.

For FlowFlow: `ContentState` carries `startedAt` (so the widget renders a self-ticking timer) and `isPaused`. There is no need to push the elapsed integer; the widget computes it from `startedAt`.

## Dynamic Island Layouts

A widget exposes four layouts via `DynamicIsland`:

| Layout | When | What to render |
|--------|------|----------------|
| Compact leading | Default collapsed view, left side | Mic icon (red, filled) |
| Compact trailing | Default collapsed view, right side | Timer (MM:SS) |
| Minimal | Multiple activities active | Small mic icon |
| Expanded | User long-press | Large timer + icon + label |

## Timer (Self-Refreshing Widget)

`Text(context.state.startedAt, style: .timer)` renders a self-ticking timer inside the widget. Combined with `.contentTransition(.numericText())`, the timer animates smoothly without the app pushing one update per second. This bypasses ActivityKit's ~1 update/second throttle and is the recommended pattern for recording timers.

Add a `startedAt: Date` field to `ContentState` alongside `isPaused`. Update it only when the user resumes (so the timer restarts from the resume instant).

## Dioxus FFI Bridge

The Rust side cannot call `Activity.request()` directly. The bridge invokes a Swift entry point that lives in the host app target (NOT the widget extension). `manganis::ffi` is still experimental in Dioxus 0.7; use raw `objc2` bindings or `@_cdecl` Swift exports with `extern "C"` Rust declarations.

The geolocation example in the Dioxus repo uses an `@objc` Swift class with methods that return JSON-encoded result strings. This avoids passing complex Swift values across the FFI boundary. Mirror that pattern for FlowFlow.

Recommended shape:

```rust
extern "C" {
    fn flowflow_start_live_activity() -> *const c_char;
    fn flowflow_update_live_activity(elapsed: u32, is_paused: bool) -> *const c_char;
    fn flowflow_end_live_activity() -> *const c_char;
}
```

Matching Swift in the host app (`src/ios/plugin/`):

```swift
import ActivityKit

@_cdecl("flowflow_start_live_activity")
public func flowflow_start_live_activity() -> UnsafePointer<CChar> {
    return strdup("{\"status\":\"ok\"}")
}
```

Swift 6 note: `Activity` is not `Sendable`. Wrap calls into `Task { @MainActor in ... }` to satisfy strict concurrency.

## Constraints

- ContentState payload size: 4 KB max.
- Update frequency: ~1 push per second sustainable, bursts allowed.
- Activity lifetime: 8 hours active by default (controlled by `staleDate`), 4 hours after `end`.
- Device support: iPhone 14 Pro+ for Dynamic Island. Lock Screen Live Activities work on all iOS 16.1+ devices.
- `Activity.request()` is foreground only. FlowFlow always starts the activity from the foreground, so this is not a blocker.
- Widget Extension provisioning: needs a separate profile from the host app. On a free Apple Developer account, this means two 7-day profiles instead of one. Anticipate the re-signing cadence.

## References

- Apple docs: <https://developer.apple.com/documentation/activitykit>
- WidgetKit docs: <https://developer.apple.com/documentation/widgetkit>
- Dioxus PR #4842 (Widget Extension support): <https://github.com/DioxusLabs/dioxus/pull/4842>
- Sample: `examples/01-app-demos/geolocation-native-plugin/` in the Dioxus repo (uses the `src/ios/plugin/` + `src/ios/widget/` split)
