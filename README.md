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
- **Audio playback** — play, pause, and delete voice recordings directly from any note
- **RAG chat** — powered by [rig](https://github.com/0xPlaygrounds/rig), ask questions about your notes and get answers with tappable source references
- **Agent tools** — the chat agent can search notes by meaning, create new notes, or summarize entire folders autonomously
- **Auto-tagging** — LLM generates single-word tags per note, or add your own
- **Document import** — PDF (with native OCR for scanned documents), DOCX, TXT, CSV via native iOS picker
- **Multi-provider** — OpenAI or Anthropic as LLM backend, switchable in settings
- **Filler removal** — auto-strips hesitations (euh, um, hmm) from transcriptions so your notes read clean
- **Local-first** — SQLite for metadata, LanceDB for semantic search, your data stays on device

## Stack

100% Rust. Zero JavaScript.

The UI runs on [Dioxus](https://github.com/DioxusLabs/dioxus), a React-like framework for Rust that renders natively on iOS through WKWebView. Styling is handled by [Tailwind CSS V4](https://tailwindcss.com) — Dioxus auto-detects and compiles it, so every class just works without a separate build step.

The RAG pipeline is built on [rig](https://github.com/0xPlaygrounds/rig), an LLM orchestration framework for Rust. It handles agent construction, tool calling, and provider dispatch (OpenAI and Anthropic) in a unified API. The agent gets custom tools — search notes, create notes, summarize folders — and can chain up to 4 tool calls per question before answering.

Embeddings go through OpenAI's text-embedding-3-small and land in [LanceDB](https://lancedb.com), a local vector database that runs entirely on device. Cosine similarity search over chunked notes and documents, no server needed.

Document import uses Apple's native PDFKit on iOS — text extraction with automatic OCR fallback for scanned documents. DOCX parsing is handled directly via zip + quick-xml, no external dependencies.

Everything async runs on [tokio](https://tokio.rs) — audio recording, API calls, embedding jobs, transcription polling. The iOS audio pipeline uses cpal for CoreAudio capture and hound for WAV encoding.

|               |                                                                                                |
| ------------- | ---------------------------------------------------------------------------------------------- |
| UI            | [Dioxus 0.7](https://github.com/DioxusLabs/dioxus) (iOS, desktop, web)                         |
| Styling       | [Tailwind CSS V4](https://tailwindcss.com)                                                     |
| LLM           | [rig-core 0.36](https://github.com/0xPlaygrounds/rig) (OpenAI + Anthropic)                     |
| Embeddings    | OpenAI text-embedding-3-small (1536 dims)                                                      |
| Vector DB     | [LanceDB 0.27.2](https://lancedb.com) (local, cosine)                                          |
| Async         | [tokio](https://tokio.rs)                                                                      |
| Database      | SQLite ([rusqlite](https://github.com/rusqlite/rusqlite), bundled, WAL mode)                   |
| Audio         | [cpal](https://github.com/RustAudio/cpal) + [hound](https://github.com/ruuda/hound)            |
| Transcription | [Soniox](https://soniox.com) REST API                                                          |
| PDF           | Apple PDFKit (iOS, native OCR) / [pdf-extract](https://crates.io/crates/pdf-extract) (desktop) |
| DOCX          | [quick-xml](https://crates.io/crates/quick-xml) + [zip](https://crates.io/crates/zip)          |
| Icons         | [Phosphor](https://phosphoricons.com) (MIT)                                                    |

## How it works

```
Talk → Record → Transcribe → Clean fillers → Auto-embed → Store

Later: Ask a question → Embed query → Vector search → Build context → Agent with tools → Answer with sources
```

## Setup

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo install dioxus-cli
cp .env.example .env   # add your API keys (or set them in-app)
```

```bash
make dev      # iOS simulator
make ddev     # physical iPhone (USB or Wi-Fi)
make desktop  # macOS desktop
make check    # fmt + clippy
```

API keys can be set in-app via Settings (stored in SQLite, no recompile). OpenAI is always required for embeddings. Anthropic is optional.

## iOS Device Provisioning

Running on a physical iPhone requires a valid provisioning profile. Apple enforces code signing for all device builds.

**Free Apple account**: profiles expire after 7 days. **Paid Apple Developer Program** (€99/year): profiles last 1 year, plus App Store and TestFlight access.

Generate or renew a provisioning profile from the terminal (no Xcode GUI needed):

```bash
# 1. Create a minimal throwaway project
mkdir -p /tmp/ffprov/flowflow.xcodeproj /tmp/ffprov/flowflow
cat > /tmp/ffprov/flowflow/flowflowApp.swift << 'EOF'
import SwiftUI
@main struct flowflowApp: App { var body: some Scene { WindowGroup { Text("ok") } } }
EOF

# 2. Get your team ID (shown in parentheses)
security find-identity -v -p codesigning | grep "Apple Development"

# 3. Get your device UDID
xcrun xctrace list devices | grep iPhone

# 4. Build to device — this generates the provisioning profile
xcodebuild -project /tmp/ffprov/flowflow.xcodeproj \
  -scheme flowflow \
  -destination "id=YOUR_DEVICE_UDID" \
  -allowProvisioningUpdates \
  -allowProvisioningDeviceRegistration \
  CODE_SIGN_STYLE=Automatic \
  DEVELOPMENT_TEAM=YOUR_TEAM_ID \
  GENERATE_INFOPLIST_FILE=YES

# 5. Verify the profile was created
ls ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/
```

The `project.pbxproj` file for step 1 is included at `flowflow/flowflow.xcodeproj/project.pbxproj`.

**On your iPhone** (first time only): Settings → General → VPN & Device Management → tap your developer certificate → Trust.

After provisioning, `make ddev` will work.

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
