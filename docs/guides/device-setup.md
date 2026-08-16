# Physical device setup and manual commands

One-time setup for running FlowFlow on a real iPhone, plus the simulator and device
commands you occasionally need by hand. Day-to-day builds go through the Makefile
(`make all`, `make ddev`) — see `CLAUDE.md`.

## One-time device setup

1. iPhone: Settings -> Privacy & Security -> Developer Mode -> enable -> restart
2. Connect via USB, accept "Trust This Computer"
3. `xcrun devicectl manage pair --device <ID>` (fixes the "no DDI" error)
4. Xcode -> Settings -> Apple Accounts -> click account -> Manage Certificates -> + -> Apple Development
5. If the certificate is not recognized by codesigning, install the Apple WWDR intermediate cert:
   `curl -sO https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer && security add-certificates AppleWWDRCAG3.cer && rm AppleWWDRCAG3.cer`
6. Verify: `security find-identity -v -p codesigning` must show "Apple Development"
7. Create a provisioning profile (required for a free Apple account) — see below
8. After the first pairing, Wi-Fi works (same network)

### Step 7: the provisioning profile workaround

This method is cumbersome and we are still looking for a simpler one. There is no
existing Dioxus issue for it — worth opening one if `dx` does not improve this.
Related App Store issue: <https://github.com/DioxusLabs/dioxus/issues/3817>

For now, create a TEMPORARY Swift/SwiftUI project in Xcode:

- Xcode -> File -> New -> Project -> iOS -> App
- Product Name: `flowflow`, Organization Identifier: `com.mirkobozzetto`, Team: Personal Team
- Interface: SwiftUI, Language: Swift (does not matter, it is temporary)
- Save to /tmp
- Select the iPhone as destination at the top of Xcode
- Cmd+R to build — Xcode creates the provisioning profile automatically
- Trust the dev profile on the iPhone: Settings -> General -> VPN & Device Management -> Trust
- Close the Xcode project (the profile stays in `~/Library/Developer/Xcode/UserData/Provisioning Profiles/`)

The profile is tied to the bundle ID (`com.mirkobozzetto.flowflow`), not to the
language. Once created, delete the Xcode project: the profile persists and
`dx serve` uses it for the Rust app.

## Manual commands

```bash
# Simulator management
open /Applications/Xcode.app/Contents/Developer/Applications/Simulator.app
xcrun simctl boot "iPhone 17 Pro"
xcrun simctl shutdown all

# Device management
xcrun devicectl list devices
xcrun devicectl manage pair --device <DEVICE_ID>
```
