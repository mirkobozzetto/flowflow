# FlowFlow

100% Rust mobile app for voice notes with transcription, folder organization, and AI features.

Built with Dioxus 0.7 for iOS. Inspired by [SuperPowerNotes](https://github.com/mirkobozzetto/superpowernotes).

## Features

- **Unified note editor** — text + voice in a single view, auto-save on exit
- **Voice dictation** — tap to record, real-time audio visualization (12-bar voice-reactive waveform)
- **Transcription** — Soniox REST API (stt-async-v4, optimized for French)
- **Folder management** — create, rename, delete folders and subfolders from sidebar drawer
- **Folder assignment** — assign notes to folders via inline picker, auto-assign from folder context
- **Note deletion** — with protection against auto-save re-creation
- **Slide transitions** — iOS-style push/pop animations (150ms)
- **Phosphor + Josemi icons** — SVG icon system (monochrome gray/blue/white palette)
- **iOS keyboard toolbar hidden** — objc2 method swizzle on WKContentView
- **SQLite storage** — local-first, persists across sessions (WAL mode)
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
| Async | tokio 1 + futures-timer 3 |
| Icons | Phosphor Icons + Josemi Icons (MIT) |
| Target | iOS (aarch64-apple-ios) |

## Architecture

```
src/
  main.rs              entry point
  models/              domain entities (Note, Folder)
  db/                  persistence (Database, migrations, CRUD)
  services/            business logic (audio, transcription)
  platform/            iOS-specific (AVAudioSession, keyboard hack)
  ui/                  Dioxus components + Tailwind
    mod.rs             App, state, routing, slide transitions
    icons.rs           SVG icon components (Phosphor + Josemi)
    layout.rs          TopBar, sidebar drawer, folder tree, FAB
    notes.rs           notes list, card, detail editor
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

### Done

- [x] Track A — Dioxus iOS scaffold
- [x] Track B — Audio capture (cpal + hound)
- [x] Track C — Soniox transcription (French, batch quality)
- [x] Track D — SQLite + folders + UI refactor + Tailwind
- [x] Unified note editor (text + inline voice dictation)
- [x] Auto-save on exit (use_drop)
- [x] Folder management (create/rename/delete/subfolders)
- [x] Folder assignment + picker in note editor
- [x] Note deletion
- [x] Slide transitions (push/pop iOS style)
- [x] Phosphor + Josemi icon system
- [x] Voice-reactive audio visualization (12 bars, real-time RMS)
- [x] iOS keyboard toolbar hidden (objc2 swizzle)
- [x] Monochrome design system (gray/blue/white)

### Next

- [ ] Audio-reactive bars: FFT frequency analysis (crate `rustfft`) for more organic visualization
- [ ] Auto-save transcription result to note content after dictation completes
- [ ] Drag-and-drop notes between folders
- [ ] Search notes (full-text search on title + content)
- [ ] Track E — On-device embeddings (ONNX, all-MiniLM-L6-v2)
- [ ] Track F — RAG + Chat (embed > search > context > LLM)
- [ ] AI-generated titles and tags
- [ ] Dark mode (optional, monochrome palette ready)
- [ ] Export notes (PDF, markdown)

## License

Private project by Mirko Bozzetto.
