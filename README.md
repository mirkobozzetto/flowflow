<p align="center">
  <img src="docs/logo.png" alt="FlowFlow" width="160" />
</p>

<h1 align="center">FlowFlow</h1>

<p align="center">
  Voice notes app for mobile - 100% Rust, built with <a href="https://github.com/DioxusLabs/dioxus">Dioxus</a>.<br/>
  Featured on <a href="https://dioxuslabs.com/awesome/">Awesome Dioxus</a>.
</p>

---

Most of my ideas come when I'm walking or between two tasks. And they vanish just as fast.

FlowFlow is a voice notes app that captures what you say, transcribes it, and lets you **chat with your notes** later. You ask "what was that pricing idea?" and the right passages come up - with links to the original notes.

No manual searching. No folders to dig through. Just talk, and find it later.

## What it does

- **Voice capture** - tap to record, real-time waveform, pause/resume, auto-transcription via Soniox or fully on-device with local Whisper models, background audio with a Dynamic Island live timer
- **Offline transcription** - download a Whisper model (tiny to large-v3-turbo) in Settings and transcribe in airplane mode; audio never leaves the device, sha256-verified downloads
- **Multi-device sync** - encrypted LAN P2P between iPhone and Mac (Noise protocol, no server, no cloud), QR pairing, real-time UI refresh, zero data loss by design
- **Backup & restore** - export everything (notes, audio, vectors) as one archive via the share sheet, API keys and pairing secrets never leave the device; import is a validated, crash-safe atomic replace, and sync reconverges without resurrecting deleted notes
- **RAG chat** - powered by [rig](https://github.com/0xPlaygrounds/rig): ask questions about your notes, get answers with tappable sources; hybrid search (BM25 + vector + RRF + LLM rerank), temporal queries, folder-scoped chat, agent tools
- **Smart reminders** - notes like "pick up the kids at 5pm" become iOS reminders, detected by LLM with one-tap confirm
- **AI organization** - auto-title while you write, single-word auto-tags, folders with hierarchy
- **Document import** - PDF (native OCR for scans), DOCX, TXT, MD, CSV via the native iOS picker, auto-embedded for chat
- **Audio playback** - play, pause, delete recordings from any note; filler words (euh, um) auto-stripped from transcripts
- **Bilingual** - English + French UI, auto-detected, switchable in Settings
- **Local-first** - SQLite for metadata, LanceDB for vectors, your data stays on your devices

## Stack

100% Rust. Zero JavaScript. The UI is [Dioxus](https://github.com/DioxusLabs/dioxus) rendering natively on iOS and macOS; the Dynamic Island Live Activity is the only Swift, bridged via FFI.

|               |                                                                                                |
| ------------- | ---------------------------------------------------------------------------------------------- |
| UI            | [Dioxus 0.7.9](https://github.com/DioxusLabs/dioxus) (iOS, desktop)                            |
| Styling       | [Tailwind CSS V4](https://tailwindcss.com)                                                     |
| LLM           | [rig-core 0.36](https://github.com/0xPlaygrounds/rig) (OpenAI, Anthropic)                      |
| Embeddings    | OpenAI text-embedding-3-small (1536 dims)                                                      |
| Vector DB     | [LanceDB 0.27.2](https://lancedb.com) (local, cosine)                                          |
| Database      | SQLite ([rusqlite](https://github.com/rusqlite/rusqlite) 0.34, bundled, WAL mode)              |
| Sync          | LAN P2P, [snow](https://github.com/mcginty/snow) (Noise XXpsk3), version vectors, tombstones   |
| Audio         | [cpal](https://github.com/RustAudio/cpal) 0.17 + [hound](https://github.com/ruuda/hound) 3.5   |
| Transcription | [Soniox](https://soniox.com) REST API or local [whisper-rs](https://github.com/tazz4843/whisper-rs) 0.16 (Metal) |
| PDF / DOCX    | Apple PDFKit (iOS, OCR) / [pdf-extract](https://crates.io/crates/pdf-extract); quick-xml + zip |
| Async         | [tokio](https://tokio.rs)                                                                      |
| Min iOS       | 16.0 (aarch64-apple-ios)                                                                       |

## How it works

```
Talk → Record → Transcribe → Clean fillers → Auto-embed → Store → AI title (1-3 words)

Later: Ask → Embed query → Hybrid search (BM25 + vector + RRF)
     → LLM rerank → Temporal boost → Tag-enriched context → Agent with tools → Answer with sources

Sync:  Save → debounced trigger → Noise-encrypted LAN session → version-vector merge → UI refresh < 1 s

Backup: Export → scrubbed SQLite snapshot + WAV + manifest (zip) → share
        Import → read-only validation → atomic swap at next launch → vector index rebuilt offline
```

## Setup

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo install dioxus-cli
cp .env.example .env   # add your API keys (or set them in-app)
```

API keys can be set in-app via Settings (stored in SQLite, no recompile). OpenAI is required for AI features; transcription works either with a Soniox key (cloud) or a downloaded local Whisper model (offline, no account).

## Commands

```bash
make all          # build + sign + icon + install on iPhone
make ddev         # dx serve --ios --device (hot reload)
make desktop-app  # build + install the Mac app
make check        # fmt check + clippy
make appstore     # release build + signed IPA
```

Full command list in the [Makefile](Makefile).

## Documentation

Everything beyond this page lives in [`docs/INDEX.md`](docs/INDEX.md):

| Chapter | What you find there |
|---------|---------------------|
| [01 Product](docs/INDEX.md#01-product) | Vision, features, history of what shipped |
| [02 Architecture](docs/INDEX.md#02-architecture) | Modules, pipelines, data model |
| [03 Dev guides](docs/INDEX.md#03-dev-guides) | Build, device setup, tests |
| [04 App Store](docs/guides/appstore.md) | Provisioning, signing, submission, troubleshooting |
| [05 Specs](docs/INDEX.md#05-specs) | RFCs and PRDs (sync, backup, realtime UX) |
| [06 History](docs/HISTORY.md) | Chronological log of every milestone |

## Tests

```bash
cargo test
cargo test -- --ignored
```

## Status

Actively developed. The codebase evolves constantly - new features, better architecture, and deeper iOS integration land regularly.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright 2026 Mirko Bozzetto - [EUPL v1.2](LICENSE)
