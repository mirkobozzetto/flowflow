# App Store media: screenshots and app previews

The reusable process for every release. Each release file (`2.1.1.md`, ...)
lists its own shots and links back here; the history at the bottom keeps
what each release shipped.

## Apple's requirements

Checked 2026-10-07 on developer.apple.com, App Store Connect Help:
"Screenshot specifications" and "App preview specifications". Check again
before each release, Apple changes them.

| Item | Rule |
| --- | --- |
| iPhone screenshots | 1 to 10 per locale, PNG or JPEG. The 6.9" slot is the only one required: 1320 x 2868 (or 1290 x 2796, 1260 x 2736) portrait. Smaller sizes are scaled from it. |
| iPad screenshots | Not needed: FlowFlow is iPhone only (`UIDeviceFamily` = `[1]`). |
| App previews (video) | Optional. Up to 3 per locale, 15 to 30 s, 886 x 1920 portrait for 6.9", H.264 (High Profile, up to Level 4.0) or ProRes 422 HQ, at most 30 fps, at most 500 MB, `.mov` `.m4v` `.mp4`. Stereo AAC audio track. |
| Order | Previews always come before screenshots on the store. |

## Rules

- **Demo data only.** Never Mirko's notes, names, keys, Hermes address or
  devices on screen. The demo stores come from `examples/demo_store`.
- **Both locales.** English (U.S.) and French (France), one full set each,
  the app and the captions in the same language.
- **Status bar** at 9:41, full battery and signal (the capture script sets it).
- Captions above each screenshot: short, the app's own tone, no claims the
  app does not keep.

## How

```bash
# Demo stores, per locale
cargo run --example demo_store -- /tmp/flowflow-demo-en.db en
cargo run --example demo_store -- /tmp/flowflow-demo-fr.db fr

# Simulator build (rustup toolchain first: Homebrew rustc has no simulator target)
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
make restore-ios-toml
set -a && . ./.env && set +a && IPHONEOS_DEPLOYMENT_TARGET=16.0 dx build --platform ios

# Screenshots: iPhone 17 Pro Max, 6.9", headless, every slot set up by a
# debug-only watcher -> screenshots/<version>/<lang>/NN.png
scripts/capture-screenshots.sh en /tmp/flowflow-demo-en.db
scripts/capture-screenshots.sh fr /tmp/flowflow-demo-fr.db

# App preview: record the booted simulator, then cut to 15-30 s at 886 x 1920
xcrun simctl io booted recordVideo --codec=h264 /tmp/preview-en.mov
```

## Mirko checks before uploading

- [ ] Every screenshot and preview: no personal data, no real key or address.
- [ ] Every caption, description, keyword and What's New, in both languages:
      spelling, accents, tone, nothing the app does not do.
- [ ] Sizes accepted by App Store Connect (it refuses the wrong ones on upload).
- [ ] The previews play to the end, poster frame chosen.

## History

| Release | Screenshots | Previews | Notes |
| --- | --- | --- | --- |
| 2.1.0 | 5 per locale, `screenshots/2.1.0/` | none | Prepared, never submitted. |
| 2.1.1 | planned, see [2.1.1.md](2.1.1.md) | planned | First submission of the 2.1 line. |
