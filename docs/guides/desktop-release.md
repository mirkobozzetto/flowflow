# Desktop release (macOS DMG)

How to ship the Mac app so anyone can download it from GitHub Releases.

## TL;DR

```bash
make dmg       # release build + icon + plists + codesign + dist/FlowFlow-X.Y.Z-macos-arm64.dmg
make release   # make dmg + notarize + staple + gh release create vX.Y.Z with the DMG attached
```

Release notes and the per-version checklist: [../release/](../release/).

The version comes from `Cargo.toml` (`version = "X.Y.Z"`). Bump it there before releasing.

## What `make dmg` does

1. Release build via `dx build --platform desktop --release` (iOS widget stripped, same mechanism as `make desktop-app`).
2. Injects the app icon, the stable bundle id (`com.mirkobozzetto.flowflow`) and the privacy usage strings (microphone, local network, Apple Events).
3. Stamps `CFBundleShortVersionString` with the Cargo version.
4. Codesigns with hardened runtime:
   - **Developer ID Application** certificate if one exists in the keychain (the right one for public distribution),
   - otherwise falls back to the **Apple Development** certificate.
   - Entitlements from `macos/entitlements.plist`: the hardened runtime
     blocks the microphone and Apple Events unless they are declared.
     DMGs up to v2.0.1 lacked them, so dictation failed for DMG installs.
5. Packages a drag-to-Applications DMG in `dist/`.

## Gatekeeper: what downloaders see

| Signing | First launch experience |
|---------|------------------------|
| Apple Development | macOS blocks the app: right-click the app > Open > Open, once. Or `xattr -d com.apple.quarantine /Applications/Flowflow.app`. |
| Developer ID + notarization (default since v2.1.0) | Opens normally, no warning. |

## One-time setup on a new Mac

1. The **Developer ID Application** certificate (team R477R8NK27) must be
   in the keychain: https://developer.apple.com/account > Certificates.
   `make dmg` picks it up automatically.
2. Store the notarization credentials once, from `.env`:
   ```bash
   set -a && . ./.env && set +a
   xcrun notarytool store-credentials flowflow-notary \
     --apple-id "$APPLE_ID" --team-id R477R8NK27 --password "$APP_SPEC_PASSWORD"
   ```

Check a DMG before publishing:

```bash
xcrun stapler validate dist/FlowFlow-X.Y.Z-macos-arm64.dmg
# mount it, then:
spctl -a -vvv -t exec /Volumes/FlowFlow/Flowflow.app   # source=Notarized Developer ID
```

## Publishing

`make release` tags `vX.Y.Z`, creates the GitHub release with auto-generated notes and attaches the DMG. Edit the notes on GitHub afterwards if needed. The README Download section points to the latest release.

Intel Macs are not covered (the DMG is arm64; a universal build would need an x86_64 dx build + `lipo`).
