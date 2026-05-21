# Dynamic Island - Build System RESOLVED

## Resolution (2026-05-21)

The original plan (build.rs + xcrun swift build + static linking) was superseded by Dioxus 0.7.9's official pattern: `#[manganis::ffi]` macro + automatic compilation by dx CLI.

No build.rs needed. dx scans SwiftPackageMetadata embedded in the binary by the manganis::ffi macro, then compiles Swift sources into DioxusSwiftPlugins.framework automatically.

## What was done

- RecordingPlugin.swift: refactored from @_cdecl free functions to @objc class with instance methods (1 arg max per method to avoid objc2 msg_send multi-arg limitation)
- Package.swift: added .static type + linkedFramework for ActivityKit/Foundation
- live_activity.rs: replaced extern "C" with #[manganis::ffi("src/ios/plugin")] + OnceLock singleton
- audio.rs: update(bool) replaced by pause()/resume()
- Cargo.toml: added manganis = "0.7"

## Build status

- aarch64-apple-ios: 0 errors
- aarch64-apple-ios-sim: 0 errors
- make check: clean

## Next

Test with make ddev on physical device. The DioxusSwiftPlugins.framework will be compiled and embedded by dx at serve/build time.
