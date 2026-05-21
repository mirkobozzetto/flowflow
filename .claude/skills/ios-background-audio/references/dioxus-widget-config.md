# Dioxus.toml Widget Extension Schema

Schema for the `[[ios.widget_extensions]]` table in `Dioxus.toml`, introduced by Dioxus PR #4842 (shipped in 0.7.4). Used by Prompt 4.

## Top-level Keys

```toml
[ios]
deployment_target = "16.0"
background_modes = ["audio"]

[ios.plist]
NSSupportsLiveActivities = true

[[ios.widget_extensions]]
source = "src/ios/widget"
display_name = "FlowFlow Recording"
bundle_id_suffix = "recording-widget"
deployment_target = "16.2"
module_name = "RecordingPlugin"
```

`[ios]` keys:

| Key | Type | Meaning |
|-----|------|---------|
| `deployment_target` | string | Minimum iOS version for the host app. FlowFlow uses `"16.0"`. |
| `background_modes` | string[] | Maps to `UIBackgroundModes` in the generated Info.plist. Use `["audio"]` to keep recording alive in background. This is the canonical PR #4842 path; do NOT set `UIBackgroundModes` manually under `[ios.plist]`. |

`[[ios.widget_extensions]]` keys:

| Key | Type | Required | Meaning |
|-----|------|----------|---------|
| `source` | string (path) | yes | Directory containing the widget's Swift sources. Relative to the project root. |
| `display_name` | string | yes | User-visible name of the widget bundle. Shown in iOS Settings. |
| `bundle_id_suffix` | string | yes | Appended to the host app bundle id. Final: `com.mirkobozzetto.flowflow.<suffix>`. |
| `deployment_target` | string | yes | Minimum iOS version for the widget. Must be 16.2+ for Dynamic Island layouts. |
| `module_name` | string | yes | Swift module name. Used by Dioxus to generate the Xcode build phase and the FFI glue. |

## What Dioxus Generates

When `dx build --features mobile --release` runs, the toolchain:

1. Compiles Swift sources under `source` into a Widget Extension target via `xcrun swiftc`, producing a static lib that is linked into the extension binary.
2. Sets the deployment target and bundle id from the table.
3. Embeds the widget extension into the host app bundle under `PlugIns/<module_name>.appex`.
4. Code-signs the widget with a SEPARATE provisioning profile (one per extension target). On a free Apple Developer account this means two 7-day profiles (host + widget) that must be re-signed weekly.

## Constraints

- `source` must contain at least one `.swift` file declaring a `@main` `WidgetBundle`.
- `bundle_id_suffix` must be unique across all widget extensions.
- `deployment_target` cannot be lower than the host app's `IPHONEOS_DEPLOYMENT_TARGET` minus extension flexibility. Practical rule: keep it equal or higher (16.2 widget on 16.0 app is fine).
- Multiple `[[ios.widget_extensions]]` tables are allowed; each emits one `.appex`.

## Verification

After a successful build, inspect the produced `.app` bundle:

```bash
ls target/dx/flowflow/release/mobile/ios/flowflow.app/PlugIns/
```

You should see `RecordingPlugin.appex` (or whatever `module_name` was set to). Inside the `.appex`, `Info.plist` should list `NSExtensionPointIdentifier = com.apple.widgetkit-extension`.

## References

- Dioxus PR #4842: <https://github.com/DioxusLabs/dioxus/pull/4842>
- Apple Widget Extension docs: <https://developer.apple.com/documentation/widgetkit/creating-a-widget-extension>
- Sample project in the Dioxus repo: `examples/01-app-demos/geolocation-native-plugin/`
