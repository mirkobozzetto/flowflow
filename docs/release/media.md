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

# Screenshots and previews: iPhone 17 Pro Max, 6.9", headless, every slot
# set up by a debug-only watcher, Hermes served by the demo's fake server
# -> screenshots/<version>/<lang>/NN.png and preview-*.mp4 (886 x 1920)
scripts/capture-screenshots.sh en /tmp/flowflow-demo-en.db
scripts/capture-screenshots.sh fr /tmp/flowflow-demo-fr.db

# Captions above each screenshot, one per line in
# screenshots/<version>/captions-<lang>.txt -> <lang>/store/NN.png
scripts/frame-screenshots.sh en
scripts/frame-screenshots.sh fr
```

Upload the `store/` frames, not the raw captures.

## Promo video (site, social)

A 35 s vertical promo per locale, edited from the same simulator clips:
`screenshots/<version>/promo/`. The editing project lives outside this repo
(`~/code/flowflow-videos`, Remotion): scenes in `src/scenes.ts`, every music
prompt tried in `music/CATALOG.md`.

```bash
# Clips: one per feature, raw takes into the video project
scripts/capture-screenshots.sh en /tmp/flowflow-demo-en.db ~/code/flowflow-videos/public/clips
ONLY="first-note" scripts/capture-screenshots.sh ...   # retake one clip

# In ~/code/flowflow-videos
scripts/prep-clips.sh en fr
npx remotion render promo-fr out/flowflow-promo-fr.mp4
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
| 2.1.1 | 8 per locale, captioned, `screenshots/2.1.1/<lang>/store/` | 2 per locale: dictation, Hermes reading a photo | First submission of the 2.1 line. Skills shown through `/`: the native `+` menu cannot open headless. Plus a 35 s promo per locale in `promo/` (funk track, Remotion). |
