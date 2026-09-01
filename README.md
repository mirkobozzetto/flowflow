<p align="center">
  <img src="docs/logo.png" alt="FlowFlow" width="160" />
</p>

<h1 align="center">FlowFlow</h1>

<p align="center">
  Voice notes that think with you - iPhone and Mac, mainly Rust, built with <a href="https://github.com/DioxusLabs/dioxus">Dioxus</a>.<br/>
  Featured on <a href="https://dioxuslabs.com/awesome/">Awesome Dioxus</a>.
</p>

---

Most of my ideas come when I'm walking or between two tasks. And they vanish just as fast.

FlowFlow is a voice notes app that captures what you say, transcribes it, and lets you **chat with your notes** later. You ask "what was that pricing idea?" and the right passages come up - with links to the original notes.

No manual searching. No folders to dig through. Just talk, and find it later.

## Get it

- **iOS** - on the App Store (v2.0), available worldwide.
- **macOS (Apple Silicon)** - grab the DMG from the [latest release](https://github.com/mirkobozzetto/flowflow/releases/latest), drag FlowFlow to Applications. First launch: right-click > Open (the build is not notarized yet).
- **From source** - see [Build](#build) below; one `make` installs on your own iPhone or Mac.

## Highlights

**Capture**

- Tap to record: real-time waveform, pause/resume, Dynamic Island live timer
- Cloud (Soniox) or fully offline transcription with local Whisper models
- Personal dictionary: your names and brands spelled right, everywhere

**Ask your notes**

- RAG chat with tappable sources: hybrid search (BM25 + vector + rerank)
- Chat through an OpenAI API key or a ChatGPT subscription; embeddings still
  use the OpenAI API
- Optional web search ([Exa](https://exa.ai)) fused into the same answer
- Save any answer or thread back as a note

**Organized for you**

- AI titles, tags and themes as you write; searchable chats
- Threads: related notes as one chronological story
- Smart filters in the search bar: dictated, reminder, document, thread

**Your devices, one brain**

- Encrypted LAN P2P sync (Noise protocol): no server, no cloud
- One account for up to 3 paired devices; premium follows the pairing automatically
- One-archive backup, validated crash-safe atomic restore

**Act, not just record**

- "Pick up the kids at 5pm" becomes a calendar event, one tap to confirm
- Notes as actions: the assistant executes with your connected tools, every write holds for approval
- Governed connectors (Google Sheets), groundwork for a signed-agent marketplace

**Native feel**

- A real Mac app: ⌘N, ⌘F, ⌘⌘, view history, native file dialogs
- 60fps waveform, edge-swipe drawers, drag-to-dismiss sheets
- English + French, down to error messages; word-level transcript with tap-to-seek

The full tour, one paragraph per feature: [docs/FEATURES.md](docs/FEATURES.md).

## How it works

```
Talk   → Record → Transcribe (cloud or on-device) → Clean fillers → Apply dictionary → Auto-embed → Store → AI title

Ask    → Embed query → Hybrid search (BM25 + vector)  ∥  Web search (Exa, when enabled)
       → RRF fusion → LLM rerank → Temporal boost → Tag-enriched context → Agent with tools → Answer with sources

Sync   → Save → debounced trigger → Noise-encrypted LAN session → version-vector merge → UI refresh < 1 s

Backup → Export scrubbed SQLite snapshot + WAV + manifest (zip) → share
         Import → read-only validation → atomic swap at next launch → vector index rebuilt offline
```

## Built with

Mainly Rust: the app itself is Rust end to end, UI included, with a few
deliberate exceptions where another tool does the job better - a small
TypeScript layer for webview gestures, two Swift packages for the Live
Activity, and the web sites in Astro.

| Part | Stack |
| ---- | ----- |
| App (`src/`) | Rust: [Dioxus 0.7](https://github.com/DioxusLabs/dioxus), [rig](https://github.com/0xPlaygrounds/rig), [LanceDB](https://lancedb.com), [rusqlite](https://github.com/rusqlite/rusqlite), [snow](https://github.com/mcginty/snow) (Noise XXpsk3), [cpal](https://github.com/RustAudio/cpal), [whisper-rs](https://github.com/tazz4843/whisper-rs), [tokio](https://tokio.rs), Tailwind CSS v4 |
| Webview gestures | ~13 KB of TypeScript (`src/ui/hooks/*.ts`), compiled by `make js` |
| Live Activity | Swift (`src/ios/widget`, `src/ios/plugin`), bridged via FFI |
| Account site (`account/`) | Astro - passkeys, plan and devices at account.flowflow.be |
| Landing (`landing-page/`) | Astro - flowflow.be |
| Backend | Rust (separate repo) - accounts, entitlements, governed connector proxy |
| AI services | OpenAI (embeddings + API-key chat), ChatGPT subscription (chat), Anthropic (chat), Soniox (cloud STT), Exa (web search) |
| Targets | iOS 16+ (aarch64-apple-ios), macOS (Apple Silicon) |

## Build

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo install dioxus-cli
cp .env.example .env   # add your API keys (or set them in-app)
```

API keys can be set in-app via Settings and are stored in SQLite. ChatGPT
subscription tokens are stored in Apple Keychain. Chat can use either an
OpenAI API key or a ChatGPT subscription. An OpenAI API key remains required
for semantic search and embeddings. Transcription works with a Soniox key or a
downloaded local Whisper model.

```bash
make all          # build + sign + icon + install on iPhone
make ddev         # dx serve --ios --device (hot reload)
make desktop-app  # build + install the Mac app in /Applications
make dmg          # distributable Mac DMG in dist/
make release      # make dmg + publish as a GitHub release
make check        # fmt check + clippy
make appstore     # release build + signed IPA
```

Full command list in the [Makefile](Makefile).

## Documentation

| Where | What |
|-------|------|
| [docs/INDEX.md](docs/INDEX.md) | Product, architecture, dev guides, App Store |
| [docs/FEATURES.md](docs/FEATURES.md) | The full feature tour |
| [docs/HISTORY.md](docs/HISTORY.md) | Every milestone, chronologically |

## Tests

```bash
cargo test
cargo test -- --ignored    # API-key-gated integration tests
```

## Status

Actively developed. The codebase evolves constantly - new features, better architecture, and deeper platform integration land regularly.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright 2026 Mirko Bozzetto - [EUPL v1.2](LICENSE)
