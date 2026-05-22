# provision-renew

Xcode project template used by `scripts/renew-profiles.sh` to regenerate expired iOS provisioning profiles via `xcodebuild -allowProvisioningUpdates`.

## Why this exists

Apple's free Personal Team provisioning profiles expire every 7 days. Without an active paid Apple Developer Program, `dx build` cannot install the app on a physical device once the profile lapses (error `0xe8008011`). Apple only regenerates a profile inside an `xcodebuild` invocation against a valid `.xcodeproj` — there is no standalone CLI path for free accounts.

This directory holds the minimal `.xcodeproj` template that gives `xcodebuild` something legitimate to build, just so the signing path triggers and Apple issues a fresh `.mobileprovision`. The resulting binary is discarded.

## Files

```
template/
  RenewApp.xcodeproj/project.pbxproj   # __BUNDLE_ID__ + __TEAM_ID__ placeholders
  Sources/App.swift                    # 9-line SwiftUI stub, never runs
  Sources/Info.plist                   # minimal iOS app plist
```

## Usage

Never invoke directly. Use:

```bash
make renew            # regenerate both app + widget profiles
make check-profiles   # show current expiration dates
make all              # auto-runs renew when expiration < 24h
```

`scripts/renew-profiles.sh` copies `template/` to `/tmp/flowflow-renew-<bundle>/`, substitutes the placeholders, runs `xcodebuild build -allowProvisioningUpdates -allowProvisioningDeviceRegistration`, then deletes the temp directory. The renewed profile lands in `~/Library/Developer/Xcode/UserData/Provisioning Profiles/`.

## When this becomes obsolete

Once your paid Apple Developer Program is active, profiles last 1 year. `make all` will detect they are still valid and skip the renewal step. The template stays in the repo as a free-tier safety net.
