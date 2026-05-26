<p align="center">
  <img src="docs/logo.png" alt="FlowFlow" width="160" />
</p>

<h1 align="center">FlowFlow</h1>

<p align="center">
  Voice notes app for mobile — 100% Rust, built with <a href="https://github.com/DioxusLabs/dioxus">Dioxus</a>.<br/>
  Featured on <a href="https://dioxuslabs.com/awesome/">Awesome Dioxus</a>.
</p>

---

Most of my ideas come when I'm walking or between two tasks. And they vanish just as fast.

FlowFlow is a voice notes app that captures what you say, transcribes it, and lets you **chat with your notes** later. You ask "what was that pricing idea?" and the right passages come up — with links to the original notes.

No manual searching. No folders to dig through. Just talk, and find it later.

## What it does

- **Voice capture** — tap to record, real-time waveform visualization, pause/resume, auto-transcription via Soniox
- **Background audio + Dynamic Island Live Activity** — recording keeps going when you lock the phone or switch apps, with a live timer in the Dynamic Island
- **Interruption handling** — auto-pause on incoming calls / FaceTime via AVAudioSession observers, auto-resume after
- **Audio playback** — play, pause, and delete voice recordings directly from any note
- **RAG chat** — powered by [rig](https://github.com/0xPlaygrounds/rig), ask questions about your notes and get answers with tappable source references
- **Hybrid search** — BM25 keyword + vector similarity + RRF fusion + LLM reranking for precise retrieval
- **Temporal queries** — ask "what did I note yesterday?" with automatic French date detection (regex + LLM fallback)
- **Folder-scoped chat** — chat searches only within the selected folder when one is active
- **Agent tools** — the chat agent can search notes by meaning, create new notes, or summarize entire folders autonomously
- **AI auto-title** — LLM generates a 1-3 word title while you write, with pulse animation
- **Auto-tagging** — LLM generates single-word tags per note, or add your own
- **Document import** — PDF (with native OCR for scanned documents), DOCX, TXT, MD, CSV via native iOS picker
- **Multi-provider** — OpenAI or Anthropic as LLM backend, switchable in settings
- **Filler removal** — auto-strips hesitations (euh, um, hmm) from transcriptions so your notes read clean
- **Local-first** — SQLite for metadata, LanceDB for semantic search, your data stays on device

## Stack

100% Rust. Zero JavaScript.

The UI runs on [Dioxus](https://github.com/DioxusLabs/dioxus), a React-like framework for Rust that renders natively on iOS through WKWebView. Styling is handled by [Tailwind CSS V4](https://tailwindcss.com) — Dioxus auto-detects and compiles it, so every class just works without a separate build step.

The RAG pipeline is built on [rig](https://github.com/0xPlaygrounds/rig), an LLM orchestration framework for Rust. It handles agent construction, tool calling, and provider dispatch (OpenAI and Anthropic) in a unified API. The agent gets custom tools — search notes, create notes, summarize folders — and can chain up to 4 tool calls per question before answering.

Embeddings go through OpenAI's text-embedding-3-small and land in [LanceDB](https://lancedb.com), a local vector database that runs entirely on device. Cosine similarity search over chunked notes and documents, no server needed.

Document import uses Apple's native PDFKit on iOS — text extraction with automatic OCR fallback for scanned documents. DOCX parsing is handled directly via zip + quick-xml, no external dependencies.

The Dynamic Island Live Activity bridges Rust to Swift via a thin FFI layer over ActivityKit, so the recording timer renders natively while the audio pipeline stays in Rust.

Everything async runs on [tokio](https://tokio.rs) — audio recording, API calls, embedding jobs, transcription polling. The iOS audio pipeline uses cpal for CoreAudio capture and hound for WAV encoding.

|               |                                                                                                |
| ------------- | ---------------------------------------------------------------------------------------------- |
| UI            | [Dioxus 0.7.9](https://github.com/DioxusLabs/dioxus) (iOS, desktop, web)                       |
| Styling       | [Tailwind CSS V4](https://tailwindcss.com)                                                     |
| LLM           | [rig-core 0.36](https://github.com/0xPlaygrounds/rig) (OpenAI + Anthropic)                     |
| Embeddings    | OpenAI text-embedding-3-small (1536 dims)                                                      |
| Vector DB     | [LanceDB 0.27.2](https://lancedb.com) (local, cosine)                                          |
| Async         | [tokio](https://tokio.rs)                                                                      |
| Database      | SQLite ([rusqlite](https://github.com/rusqlite/rusqlite) 0.34, bundled, WAL mode)              |
| Audio         | [cpal](https://github.com/RustAudio/cpal) 0.17 + [hound](https://github.com/ruuda/hound) 3.5   |
| Transcription | [Soniox](https://soniox.com) REST API                                                          |
| Live Activity | ActivityKit via Swift FFI (Dynamic Island recording timer)                                     |
| PDF           | Apple PDFKit (iOS, native OCR) / [pdf-extract](https://crates.io/crates/pdf-extract) (desktop) |
| DOCX          | [quick-xml](https://crates.io/crates/quick-xml) + [zip](https://crates.io/crates/zip)          |
| Icons         | [Phosphor](https://phosphoricons.com) (MIT)                                                    |
| Min iOS       | 16.0 (aarch64-apple-ios)                                                                       |

## How it works

```
Talk → Record → Transcribe → Clean fillers → Auto-embed → Store → AI title (1-3 words)

Later: Ask → Embed query → Hybrid search (BM25 + vector + RRF)
     → LLM rerank → Temporal boost → Tag-enriched context → Agent with tools → Answer with sources
```

## Setup

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo install dioxus-cli
cp .env.example .env   # add your API keys (or set them in-app)
```

API keys can be set in-app via Settings (stored in SQLite, no recompile). OpenAI is always required for embeddings. Anthropic is optional.

## Commands

```bash
make all            # build + sign widget + icon + install (auto-renews expired profiles)
make ddev           # dx serve --ios --device (hot reload, no widget signing)
make dev            # dx serve --ios (simulator)
make desktop        # dx serve --desktop (Mac window, real mic)
make build          # cargo build --features mobile
make format         # cargo fmt
make check          # fmt check + clippy
make deploy         # dx build device + icon injection
make appstore       # release build + distribution signing + IPA
make renew          # regenerate iOS provisioning profiles (xcodebuild)
make check-profiles # show profile expiration dates
make logs           # open Console.app (select iPhone, filter "FlowFlow")
make clean          # rm target/dx
```

## iOS Device Provisioning

Running on a physical iPhone requires a valid provisioning profile. Apple enforces code signing for all device builds.

**Free Apple account**: profiles expire after 7 days. **Paid Apple Developer Program** (€99/year): profiles last 1 year, plus App Store and TestFlight access.

`make all` auto-detects expiry < 24h and runs `make renew` before building. Renewal uses a minimal Xcode project template under `tools/provision-renew/` invoked via `xcodebuild -allowProvisioningUpdates`. Zero manual Xcode GUI required.

```bash
make check-profiles  # show expiration dates
make renew           # regenerate now
```

**On your iPhone** (first time only): Settings → General → VPN & Device Management → tap your developer certificate → Trust.

Once your paid Apple Developer Program is active, profiles last 1 year and `make renew` becomes a once-a-year operation.

## App Store distribution

`make appstore` produces a `FlowFlow.ipa` that passes Apple's validator with zero errors. The Dioxus CLI omits several keys required for App Store distribution (`CFBundlePackageType`, `DT*`, widget `UIRequiredDeviceCapabilities`, ...) and uses a `ditto` invocation that strips the `Payload/` directory; the Makefile patches all of these. Full breakdown of every Apple validation error we hit and the corresponding fix: [docs/deploy/04-dioxus-app-store-workarounds.md](docs/deploy/04-dioxus-app-store-workarounds.md). Upstream tracking: [Dioxus issue #3817](https://github.com/DioxusLabs/dioxus/issues/3817).

End-to-end submission walkthrough: [docs/deploy/02-app-store-submission.md](docs/deploy/02-app-store-submission.md). Sequential execution plan: [docs/deploy/03-execution-plan.md](docs/deploy/03-execution-plan.md).

### Glossary

- **IPA** — iOS App Archive. A signed `.ipa` is the zipped, signed bundle Apple accepts. `make appstore` builds it.
- **ASC** — [App Store Connect](https://appstoreconnect.apple.com). Apple's web dashboard to upload builds, fill metadata, attach screenshots, and submit for review.
- **Transporter** — [free Mac app](https://apps.apple.com/app/transporter/id1450874784) to upload an `.ipa` to ASC.
- **Provisioning profile** — Apple signed file that authorizes your bundle ID + cert combo. Generated once at [developer.apple.com](https://developer.apple.com/account/resources/profiles/list).
- **CFBundleVersion / CFBundleShortVersionString** — build number (must increment every upload) and user-visible version (e.g. `1.0.0`).
- **Bundle ID** — unique app identifier (`com.mirkobozzetto.flowflow`). Tied to the provisioning profile.

### Push an update (continuous releases)

Workflow once the app is live on the Store:

1. Bump `CFBundleVersion` in `Makefile` (line 74) by `+1`. ASC rejects duplicate build numbers. Bump `CFBundleShortVersionString` only on user-visible releases (e.g. `1.0.0` → `1.0.1`).
2. `make appstore` → fresh signed `FlowFlow.ipa`.
3. Open Transporter → drag the `.ipa` → Deliver. Apple processes the build (5–30 min).
4. ASC → your app → **TestFlight or App Store tab** → attach the new build to the current version, or create a new version (`+ Version` button) if you bumped the short version string.
5. Update locale-specific metadata (description, keywords, screenshots) if needed — see *Multilingual releases* below.
6. **Submit for Review**. Apple usually answers within 24h.

### Multilingual releases

The app auto-detects the iPhone system language at boot (NSLocale) and falls back to English. Users can switch language any time via Settings → Langue / Language. Supported locales: `en`, `fr`.

For ASC: add **English (U.S.)** and **French (France)** under App Information → Localizations. Each locale needs its own screenshots (iPhone 6.9", 1320×2868), description, and keywords. Upload locale screenshots separately in each language tab.

## Tests

```bash
cargo test
cargo test -- --ignored
```

## Status

Actively developed. The codebase evolves constantly — new features, better architecture, and deeper iOS integration land regularly.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright 2026 Mirko Bozzetto — [EUPL v1.2](LICENSE)
