<p align="center">
  <img src="docs/logo.png" alt="FlowFlow" width="160" />
</p>

<h1 align="center">FlowFlow</h1>

<p align="center">
  Voice notes app for iPhone — 100% Rust, built with Dioxus.
</p>

---

Most of my ideas come when I'm walking or between two tasks. And they vanish just as fast.

FlowFlow is a voice notes app that captures what you say, transcribes it, and lets you **chat with your notes** later. You ask "what was that pricing idea?" and the right passages come up — with links to the original notes.

No manual searching. No folders to dig through. Just talk, and find it later.

## What it does

- **Voice capture** — tap to record, real-time waveform visualization, auto-transcription (Soniox)
- **RAG chat** — ask questions about your notes, AI answers with source references you can tap to jump back
- **Auto-tagging** — LLM generates 3-5 tags per note, or add your own
- **Document import** — drop PDF, DOCX, TXT, CSV files into any note via native iOS picker
- **Multi-provider** — OpenAI or Anthropic, pick in settings
- **Agent tools** — chat can search notes, create notes, and summarize entire folders
- **Filler removal** — auto-strips hesitations (euh, um, hmm) from transcriptions
- **Local-first** — SQLite for metadata, LanceDB for vector search, everything on device

## Stack

100% Rust. Zero JavaScript.

| | |
|-|-|
| UI | [Dioxus 0.7](https://dioxuslabs.com) (iOS via WKWebView) |
| Styling | Tailwind CSS V4 |
| LLM | [rig-core 0.36](https://github.com/0xPlaygrounds/rig) (OpenAI + Anthropic) |
| Embeddings | OpenAI text-embedding-3-small |
| Vector DB | [LanceDB 0.27.2](https://lancedb.com) (local, cosine) |
| Database | SQLite (rusqlite, bundled, WAL mode) |
| Audio | cpal + hound (CoreAudio on iOS) |
| Transcription | Soniox REST API |
| PDF/DOCX | pdf-extract + zip/quick-xml |
| Icons | [Phosphor](https://phosphoricons.com) (MIT) |

## How it works

```
Talk → Record → Transcribe → Auto-embed → Store

Later: Ask a question → Embed query → Vector search → Build context → Agent response with sources
```

The agent has tools: it can search notes by semantic similarity, create new notes, or summarize folders. Up to 4 tool turns per question before the final answer.

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

## Tests

```bash
cargo test                  # 101 tests
cargo test -- --ignored     # E2E (needs API keys)
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright 2026 Mirko Bozzetto — [EUPL v1.2](LICENSE)
