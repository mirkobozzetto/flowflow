# 06 — Dioxus Platform

Dioxus 0.7 capabilities, limitations, JS-in-WKWebView possibilities, objc2 patterns, and roadmap for mobile iOS development.

## Current State: Dioxus 0.7 (released Oct 31, 2025)

### What Works Well
- **Rendering**: WKWebView via wry 52 + tao (same libs as Tauri)
- **Hot-reload**: Subsecond hot-patching of Rust code on iOS device and simulator
- **CSS animations**: supported, including transparency
- **iPad**: official support added in 0.7
- **Device deployment**: `dx serve --platform ios --device` works
- **Configuration**: Info.plist and entitlements via `Dioxus.toml`
- **Bundling**: `dx bundle --ipa` for App Store submission
- **Live Activities / iOS widgets**: supported since 0.7.4 (March 2026)
- **Deep linking / URL schemes**: supported since 0.7.4

### Critical Bugs (Open Issues)

| Issue | Problem | Impact |
|-------|---------|--------|
| [#4894](https://github.com/DioxusLabs/dioxus/issues/4894) | Scroll in simulator blocks Dioxus execution for ~8s | Cannot build apps with live visual feedback on simulator |
| [#4709](https://github.com/DioxusLabs/dioxus/issues/4709) | App freeze/blank screen after background on iOS 18.5+ | WKWebView WebProcess crash, partial fix in PR #4391 |
| [#3817](https://github.com/DioxusLabs/dioxus/issues/3817) | App Store deployment broken (missing DTPlatformName) | Manual plist workaround required |

### Structural WKWebView Limitations
- No native UIKit/SwiftUI rendering — everything is HTML/CSS
- No native iOS gestures (swipe-to-delete, peek-and-pop, force-touch)
- No haptics without objc2 FFI (AudioToolbox/UIImpactFeedbackGenerator)
- No native access to: camera, photos, share sheet, file picker, push notifications, biometrics, Keychain, CoreLocation — except via objc2
- No multi-window (stubbed on mobile)
- Background tasks: only via Info.plist + FFI
- `tokio::time::sleep` doesn't work in Dioxus async — use `futures-timer::Delay`

## JavaScript in WKWebView

FlowFlow is 100% Rust, but JS CAN run inside WKWebView. This opens possibilities.

### Dioxus `eval()` API

Dioxus provides `document::eval()` to execute arbitrary JavaScript in the WebView:

```rust
use dioxus::prelude::*;

// Run JS and get result
let result = document::eval("document.title").await;

// Run JS without waiting
document::eval("console.log('hello from Rust')");

// Pass data to JS
let js = format!("window.processData({})", serde_json::to_string(&data)?);
document::eval(&js);
```

### Potential JS Libraries

| Library | Size | Use Case | Feasibility |
|---------|------|----------|-------------|
| marked.js | ~30KB | Markdown → HTML rendering | High — inject via eval, render in div |
| highlight.js | ~30KB | Code syntax highlighting | Medium — useful if markdown supports code blocks |
| DOMPurify | ~15KB | Sanitize HTML (security) | High — if rendering user HTML |
| Mermaid | ~1MB | Diagrams from text (mind maps) | Low — heavy, niche use case |

### How to Load JS Libraries

Option 1: Inline via eval (small libs)
```rust
document::eval(include_str!("../assets/marked.min.js"));
```

Option 2: Script tag in Dioxus RSX
```rust
rsx! {
    script { src: asset!("/assets/js/marked.min.js") }
}
```

Option 3: Rust-native alternative
```rust
// Use pulldown-cmark crate instead of marked.js
let html = pulldown_cmark::html::push_html(&mut output, parser);
```

### Recommendation

Prefer Rust-native crates over JS when available. Use JS only for things with no good Rust equivalent in WKWebView context. `pulldown-cmark` for markdown is better than loading marked.js.

## objc2 Patterns (iOS Native Access)

### Already Implemented in FlowFlow

| Feature | File | Lines | API |
|---------|------|-------|-----|
| AVAudioSession (PlayAndRecord) | `src/platform/ios.rs:40-54` | 15 | objc2-avf-audio |
| Hide keyboard toolbar (swizzle) | `src/platform/ios.rs:4-33` | 30 | objc2 raw FFI |
| Documents directory | `src/platform/ios.rs:35-38` | 4 | std::env HOME |

### Pattern for New iOS APIs

Standard objc2 pattern used in FlowFlow:
```rust
#[cfg(target_os = "ios")]
pub fn some_ios_feature() {
    unsafe {
        // Get class, selector, method
        // Call via objc2 FFI
        // Handle errors gracefully
    }
}
```

### Effort Estimates for New objc2 Features

| Feature | iOS API | Estimated Lines | Effort |
|---------|---------|----------------|--------|
| Photo picker | PHPickerViewController | ~80 | 1 day |
| Camera capture | UIImagePickerController | ~60 | 1 day |
| Share sheet | UIActivityViewController | ~50 | 0.5 day |
| Haptic feedback | UIImpactFeedbackGenerator | ~20 | 2 hours |
| Face ID / biometrics | LAContext | ~50 | 0.5 day |
| Push notifications | UNUserNotificationCenter | ~100 + entitlements | 1-2 days |
| File picker | UIDocumentPickerViewController | ~80 | 1 day |
| Keychain storage | Security framework | ~100 | 1 day |
| Background tasks | BGTaskScheduler + Info.plist | ~80 | 1 day |

Note: `<input type="file">` in WKWebView may handle photo/file picking without any objc2 (needs testing on real device).

### Required Crates for New iOS APIs

```toml
[target.'cfg(target_os = "ios")'.dependencies]
objc2 = "0.6"
objc2-avf-audio = "0.3"       # already present
objc2-foundation = "0.3"      # already present
objc2-ui-kit = "0.3"          # add for UIKit APIs (photo picker, share sheet)
objc2-local-authentication = "0.3"  # add for Face ID
```

## Roadmap: Dioxus 0.8 and Beyond

Source: [Discussion #5024](https://github.com/DioxusLabs/dioxus/discussions/5024)

### Confirmed for 0.8

| Feature | Status | Impact on FlowFlow |
|---------|--------|-------------------|
| Native APIs (camera, location, storage, oauth) | PR #4842 merged (0.7.4) | Replaces manual objc2 for common APIs |
| `manganis::ffi` macro (Rust↔Swift/Kotlin bindings) | Merged | Auto-generates bindings, invokes swiftc |
| Unified permissions system | In progress | `Permission::CAMERA` cross-platform |
| Deep linking / URL schemes | Merged (0.7.4) | Already available |
| Live Activities / widget extensions | Merged (0.7.4) | Already available |
| Migration tao → winit | In progress | May fix scroll bug #4894 |

### Blitz (Native Renderer — Experimental)

- **What**: HTML/CSS rendering via Vello/WGPU, replacing WKWebView
- **Status**: Beta 0.3 estimated May 2026
- **iOS**: Draft PR for UIKit renderer (objc2-based, maps divs to UIView)
- **Current limitations**: no `position:fixed`, no `<video>`, no JS, no touch events, no text editing
- **Production timeline**: ~2027 at earliest
- **Verdict**: not ready for FlowFlow. Stay on WKWebView.

### Dioxus Team Direction
- No plans for full UIKit/SwiftUI native rendering as primary path
- Main strategy: WebView + better FFI via `manganis::ffi`
- Native gestures are NOT on visible roadmap
- Community SDK (`dioxus-sdk`) has partial mobile support (storage, time) but no camera/WiFi/Bluetooth

## Comparison: Dioxus vs Alternatives

### Dioxus vs Tauri Mobile v2

| Criterion | Dioxus 0.7 | Tauri v2 |
|-----------|-----------|----------|
| iOS rendering | WKWebView (wry/tao) | WKWebView (wry/tao) — **same libs** |
| UI source | Rust RSX | JS/TS framework |
| Native FFI | objc2 + manganis::ffi | Plugin JS↔Rust IPC bridge |
| Plugin ecosystem | Nascent | More mature (dialog, fs, notification, haptics) |
| Hot-reload | Subsecond (Rust) | Vite HMR (frontend) |
| Bundle size | <15MB | <15MB |
| App Store maturity | Open bugs, plist workarounds | More stable |
| Production track record | Near zero on iOS | Several apps in production |

### Dioxus vs SwiftUI (for FlowFlow)

| Criterion | Dioxus | SwiftUI |
|-----------|--------|---------|
| Language | 100% Rust | Swift (+ Rust FFI for core libs) |
| Audio | cpal (already working) | AVAudioEngine (native) |
| Performance | WKWebView overhead | Native 60fps |
| Animations | CSS only | All native iOS animations |
| Gestures | JS touch simulation | All native iOS gestures |
| App Store | Workarounds needed | Standard Xcode |
| ONNX / LanceDB | Direct Rust | Via Swift↔Rust FFI bridge |
| Iteration speed | Hot-patch global | Xcode rebuild |

### Verdict for FlowFlow

**Stay on Dioxus** for MVP through Track F (RAG/Chat). The Rust-native stack (cpal, Soniox client, LanceDB, embeddings) is the core value — rewriting in Swift would mean maintaining a FFI bridge.

**Plan a SwiftUI port** if the app gets traction and needs polish: native gestures, haptics, App Store stability, widgets. The clean architecture (`models/`, `db/`, `services/`) makes the core portable as a Rust `staticlib`.

## Production Readiness Assessment

| Dimension | Status | Notes |
|-----------|--------|-------|
| Development velocity | Good | Subsecond hot-reload is excellent |
| iOS deployment | Fragile | Plist workarounds, background crashes |
| App Store submission | Broken | Issue #3817 open, manual fix needed |
| Performance | Adequate | WKWebView OK for <200 notes |
| Stability | Moderate | Background freeze on iOS 18.5+ |
| Ecosystem | Immature | dioxus-sdk incomplete for mobile |

**Bottom line**: viable for personal/early-stage app. Not ready for demanding users or App Store submission without workarounds.
