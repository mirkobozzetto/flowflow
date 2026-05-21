# Dynamic Island Animation Research

## Current State

Live Activity with Dynamic Island working on FlowFlow.
Static waveform icon + timer on compact leading side.
No continuous animation yet.

## Why Animations Don't Work (Current)

- `.symbolEffect(.variableColor.iterative.reversing)` applied to `waveform` SF Symbol
- Probable cause: `waveform` may not have variable color layers (not all SF Symbols do)
- Live Activities block `.animation(.repeatForever())` style continuous animations
- Apple Music uses private API for its animated waveform

## How to Implement Animation (Proven Approach)

Live Activities animate during ContentState changes (iOS 17+).
Each update triggers a 2-second animation window.
Source: https://dev.to/canopassoftware/integrating-live-activity-and-dynamic-island-in-ios-part-2-51fo

### Implementation Plan

1. Add `phase: Int` to `RecordingAttributes.ContentState`
2. From Rust, call `flowflow_update_live_activity` every 2-3 seconds during recording
3. Increment `phase` on each update
4. In widget SwiftUI, use `phase` to vary bar heights with `.animation(.smooth, value: phase)`
5. SwiftUI auto-animates the transition between states

### Rust Side Changes Needed

- Add periodic timer in `AudioRecorder` or UI layer (tokio interval or use_effect)
- Call `live_activity::update(false)` every 2-3 seconds while recording
- Modify `flowflow_update_live_activity` Swift FFI to accept `phase: Int32`
- Or: add separate `flowflow_tick_live_activity(phase: Int32)` function

### Swift Side Changes Needed

```swift
// RecordingAttributes.swift (both plugin AND widget copies)
struct ContentState: Codable, Hashable {
    var startedAt: Date
    var isPaused: Bool
    var phase: Int  // NEW: animation phase, incremented every 2-3s
}

// RecordingWidget.swift - animated bars
struct AnimatedBars: View {
    var phase: Int
    var height: CGFloat

    var body: some View {
        HStack(spacing: 1.5) {
            bar(0.4 + sin(phase, offset: 0))
            bar(0.6 + sin(phase, offset: 1))
            bar(0.9 + sin(phase, offset: 2))
            bar(0.5 + sin(phase, offset: 3))
            bar(0.7 + sin(phase, offset: 4))
        }
        .frame(height: height)
        .animation(.smooth(duration: 1.5), value: phase)
    }

    func sin(_ phase: Int, offset: Int) -> CGFloat {
        let x = Double((phase + offset) % 6) / 6.0 * .pi * 2
        return CGFloat(Foundation.sin(x)) * 0.3
    }

    func bar(_ fraction: CGFloat) -> some View {
        RoundedRectangle(cornerRadius: 1)
            .fill(.white)
            .frame(width: 2, height: max(2, height * fraction))
            .frame(height: height, alignment: .bottom)
    }
}
```

### RecordingPlugin.swift FFI Change

```swift
@_cdecl("flowflow_update_live_activity")
public func updateLiveActivity(_ startedAtUnix: Int64, _ isPaused: Bool, _ phase: Int32) {
    // update with new phase value
}
```

### Battery Considerations

- Updates every 2-3 seconds = ~20-30 updates/minute
- Apple recommends max 1 update/second for Live Activities
- 2-3 second interval is safe
- Each update is tiny (3 fields in ContentState)

## Reference Links

- Apple VariableColorSymbolEffect: https://developer.apple.com/documentation/symbols/variablecolorsymboleffect
- WWDC23 Animate Symbols: https://developer.apple.com/videos/play/wwdc2023/10258/
- rtaudio (reverse-eng Apple waveform): https://github.com/ZephyrCodesStuff/rtaudio
- Live Activity animation guide: https://dev.to/canopassoftware/integrating-live-activity-and-dynamic-island-in-ios-part-2-51fo
- DynamicIslandFramework (custom, not ActivityKit): https://github.com/shikilpk333/DynamicIslandFramework
- SO: timer takes full width fix: https://stackoverflow.com/questions/75351633
- SwiftUI audio visualizer pattern: https://cindori.com/developer/swiftui-animation-audio
- SF Symbol effects guide: https://sarunw.com/posts/animate-sf-symbols-with-symboleffect/
- Symbol effects vocabulary: https://blakecrosley.com/blog/symbol-effects-vocabulary

## Constraints

- Live Activities max animation duration: 2 seconds per update
- Always-On Display: animations disabled
- iOS 16: only system animations (.move, .slide), no custom
- iOS 17+: full SwiftUI animations during content transitions
- Continuous .repeatForever() blocked in widgets
- Apple Music waveform: private API, not reproducible exactly
