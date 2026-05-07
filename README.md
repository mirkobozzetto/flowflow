# FlowFlow

100% Rust mobile app for voice notes with transcription, folder organization, and AI features.

Built with Dioxus 0.7 for iOS. Inspired by [SuperPowerNotes](https://github.com/mirkobozzetto/superpowernotes).

## Features

- **Voice recording** — capture audio via iOS mic (cpal + CoreAudio)
- **Transcription** — Soniox REST API (stt-async-v4, optimized for French)
- **Text notes** — create and edit notes manually
- **Folders** — hierarchical organization with subfolders (N:N with notes)
- **SQLite storage** — local-first, persists across sessions
- **Tailwind CSS** — utility-first styling via Dioxus 0.7 auto-detection

## Stack

| Layer | Tech |
|-------|------|
| Language | Rust 1.94 |
| UI Framework | Dioxus 0.7 (WKWebView on iOS) |
| Styling | Tailwind CSS V4 |
| Audio | cpal 0.17 + hound 3.5 |
| Transcription | Soniox REST API (stt-async-v4) |
| Database | SQLite (rusqlite 0.34, bundled) |
| HTTP | reqwest 0.12 |
| Async | tokio 1 |
| Target | iOS (aarch64-apple-ios) |

## Architecture

```
src/
  main.rs              entry point
  models/              domain entities (Note, Folder)
  db/                  persistence (Database, migrations, CRUD)
  services/            business logic (audio, transcription)
  platform/            iOS-specific (AVAudioSession, documents dir)
  ui/                  Dioxus components + Tailwind
    mod.rs             App, state, routing
    layout.rs          TopBar, sidebar drawer, FAB
    notes.rs           notes list, card, detail
    recording.rs       recording view
```

## Setup

### Prerequisites

- macOS with Xcode installed
- Rust with iOS targets: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`
- Dioxus CLI: `cargo install dioxus-cli`

### Environment

```bash
cp .env.example .env
# Add your Soniox API key
```

Get a Soniox API key at https://console.soniox.com

### Run

```bash
make dev      # iOS simulator
make ddev     # physical iPhone (USB or Wi-Fi)
make desktop  # macOS desktop (real mic)
```

### Physical iPhone setup

1. Enable Developer Mode: Settings > Privacy & Security > Developer Mode
2. Connect via USB, trust the computer
3. Pair: `xcrun devicectl manage pair --device <ID>`
4. Create provisioning profile via temporary Xcode project (see CLAUDE.md)
5. After first pairing, Wi-Fi works on same network

## Roadmap

- [x] Track A — Dioxus iOS scaffold
- [x] Track B — Audio capture (cpal + hound)
- [x] Track C — Soniox transcription (French, batch quality)
- [x] Track D — SQLite + folders + UI refactor + Tailwind
- [ ] Wire recording flow to SQLite (auto-save after transcription)
- [ ] Folder management (create, rename, delete from sidebar)
- [ ] Track E — On-device embeddings (ONNX, all-MiniLM-L6-v2)
- [ ] Track F — RAG + Chat (embed > search > context > LLM)

## License

Private project by Mirko Bozzetto.
