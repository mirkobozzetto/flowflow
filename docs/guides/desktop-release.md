# Desktop release (macOS DMG)

How to ship the Mac app so anyone can download it from GitHub Releases.

## TL;DR

```bash
make dmg       # release build + icon + plists + codesign + dist/FlowFlow-X.Y.Z-macos-arm64.dmg
make release   # make dmg + gh release create vX.Y.Z with the DMG attached
```

The version comes from `Cargo.toml` (`version = "X.Y.Z"`). Bump it there before releasing.

## What `make dmg` does

1. Release build via `dx build --platform desktop --release` (iOS widget stripped, same mechanism as `make desktop-app`).
2. Injects the app icon, the stable bundle id (`com.mirkobozzetto.flowflow`) and the privacy usage strings (microphone, local network, Apple Events).
3. Stamps `CFBundleShortVersionString` with the Cargo version.
4. Codesigns with hardened runtime:
   - **Developer ID Application** certificate if one exists in the keychain (the right one for public distribution),
   - otherwise falls back to the **Apple Development** certificate.
5. Packages a drag-to-Applications DMG in `dist/`.

## Gatekeeper: what downloaders see

| Signing | First launch experience |
|---------|------------------------|
| Apple Development (current default) | macOS blocks the app: right-click the app > Open > Open, once. Or `xattr -d com.apple.quarantine /Applications/Flowflow.app`. |
| Developer ID + notarization | Opens normally, no warning. |

## Upgrading to frictionless installs (one-time setup)

1. Create a **Developer ID Application** certificate: https://developer.apple.com/account > Certificates > + > Developer ID Application (requires the paid Developer Program, team R477R8NK27). Download and double-click to install it in the keychain. `make dmg` picks it up automatically.
2. Store notarization credentials once:
   ```bash
   xcrun notarytool store-credentials flowflow-notary \
     --apple-id "$APPLE_ID" --team-id R477R8NK27
   ```
   (uses an app-specific password from https://account.apple.com > Sign-In and Security)
3. Notarize and staple after `make dmg`:
   ```bash
   xcrun notarytool submit dist/FlowFlow-*.dmg --keychain-profile flowflow-notary --wait
   xcrun stapler staple dist/FlowFlow-*.dmg
   ```
4. `make release` as usual.

## Publishing

`make release` tags `vX.Y.Z`, creates the GitHub release with auto-generated notes and attaches the DMG. Edit the notes on GitHub afterwards if needed. The README Download section points to the latest release.

Intel Macs are not covered (the DMG is arm64; a universal build would need an x86_64 dx build + `lipo`).
