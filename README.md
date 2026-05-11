# FlowFlow

Mobile voice notes app with AI chat — 100% Rust, Dioxus iOS, local-first (SQLite + LanceDB).

Built with Dioxus 0.7 for iOS. Inspired by [SuperPowerNotes](https://github.com/mirkobozzetto/superpowernotes).

## Features

- **Unified note editor** — text + voice in a single view, auto-save on exit via `use_drop`
- **Voice dictation** — tap to record, real-time audio visualization (12-bar waveform, 80ms polling)
- **Transcription** — Soniox REST API (stt-async-v4, French-optimized)
- **Auto-embed** — notes auto-embedded on save (>50 chars) → chunked → OpenAI → LanceDB
- **AI tags** — LLM auto-generated tags (3-5 per note) + manual add/remove chips
- **Multi-provider LLM** — OpenAI (gpt-4o-mini) or Anthropic (Claude Haiku 4.5), picked in Settings
- **RAG chat** — ask questions about your notes, AI answers with source references
- **Agent tools** — chat can search notes, create notes, and summarize folders on demand
- **Tool call status** — chat shows real-time tool activity ("Recherche dans les notes...", etc.)
- **Note search** — live search bar filters notes by title, content, and tags
- **Chat history** — persistent conversations in SQLite, sidebar tabs Notes/Chats
- **Markdown responses** — AI chat renders bold, lists, code blocks (pulldown-cmark)
- **Clickable sources** — tap a source card to jump to the referenced note, back returns to chat
- **In-app settings** — API keys stored in SQLite (no .env recompile needed)
- **Folder management** — create, rename, delete folders and subfolders from sidebar drawer
- **French auto-title** — notes auto-titled with local date ("8 mai 2026, 14:30")
- **Slide transitions** — iOS-style push/pop animations (150ms, CSS keyframes)
- **iOS keyboard handling** — visualViewport API with cached keyboard height
- **Phosphor icons** — 18 SVG icon components (monochrome gray/blue/white palette)
- **Document attachments** — import TXT, MD, CSV, PDF, DOCX into any note via native iOS picker
- **Attachment cards** — imported files appear as cards on NoteDetail with bottom-sheet modal viewer
- **Attachment delete confirmation** — Apple-style overlay (backdrop-blur, Annuler/Supprimer pills)
- **Auto-embed attachments** — imported documents chunked + embedded (chunk id scheme `att:{att_id}:{idx}`)
- **Chat menu** — 3-dots menu on chat view: rename conversation, delete with confirmation
- **Graceful audio session** — app launches even when another app holds the microphone (Messenger, Zoom)
- **SQLite storage** — local-first with WAL mode, foreign keys, versioned migrations (V1 + V2 + V3)
- **Tailwind CSS V4** — utility-first styling via Dioxus 0.7 auto-detection

## Stack

| Layer         | Tech                                                            |
| ------------- | --------------------------------------------------------------- |
| Language      | Rust 1.94                                                       |
| UI Framework  | Dioxus 0.7 (WKWebView on iOS)                                   |
| Styling       | Tailwind CSS V4                                                 |
| Audio         | cpal 0.17 + hound 3.5                                           |
| Transcription | Soniox REST API (stt-async-v4)                                  |
| Database      | SQLite (rusqlite 0.34, bundled)                                 |
| Vector DB     | LanceDB 0.27.2 (local semantic search)                          |
| LLM Framework | rig-core 0.36 (OpenAI + Anthropic providers)                    |
| Embeddings    | OpenAI API (text-embedding-3-small, 1536 dims) — always OpenAI  |
| Chat          | OpenAI gpt-4o-mini OR Anthropic Claude Sonnet 4.6 via rig agent |
| Markdown      | pulldown-cmark 0.12                                             |
| PDF parsing   | pdf-extract 0.10                                                |
| DOCX parsing  | zip 2 + quick-xml 0.36                                          |
| HTTP          | reqwest 0.13 (multipart + JSON + rustls)                        |
| Async         | tokio 1 + futures-timer 3                                       |
| Serialization | serde 1.0 + serde_json 1.0                                      |
| Date/Time     | chrono 0.4                                                      |
| UUID          | uuid 1 (v4)                                                     |
| Env           | dotenvy 0.15                                                    |
| iOS Platform  | objc2 + objc2-avf-audio                                         |
| Icons         | Phosphor Icons (MIT)                                            |
| Target        | iOS (aarch64-apple-ios, aarch64-apple-ios-sim)                  |

## Data Model

### SQLite Schema (V1 + V2 + V3 migrations)

```
notes (id, note_type, title, content, audio_file_path, duration_secs, tags[], created_at, modified_at)
folders (id, name, description, parent_id → self-ref, created_at, modified_at)
notes_folders (folder_id, note_id) — N:N junction, CASCADE on delete
settings (key PK, value) — API keys + llm_provider stored locally
conversations (id, title, created_at, modified_at)
conversation_messages (id, conversation_id → CASCADE, role, content, sources_json, created_at)
attachments (id, note_id → CASCADE, filename, content_text, imported_at) — V3, idx_attachments_note
```

### Pipelines

```
Note Pipeline:
  Mic → cpal → WAV → Soniox REST → transcription
    → SQLite + auto-embed on save
    → chunked → OpenAI embed → LanceDB (cosine)

Attachment Pipeline:
  Native iOS picker (UIDocumentPickerViewController)
    → read_file_as_text: txt/md/csv direct, pdf via pdf-extract, docx via zip + quick-xml
    → SQLite attachments table (CASCADE on parent note)
    → chunked → OpenAI embed → LanceDB (chunk id `att:{att_id}:{idx}`)

RAG Chat Pipeline:
  User question → OpenAI embed (query vector, always OpenAI)
    → LanceDB search (top 5 chunks, cosine)
    → Build context from matched notes
    → rig Agent with tools (search_notes, create_note, summarize_folder)
    → Provider dispatch: OpenAI gpt-4o-mini OR Anthropic Claude Sonnet 4.6
    → Up to 4 tool turns, then final response
    → Markdown response + source cards

API Key Fallback:
  SQLite settings → env var → option_env!() compile-time
```

## Setup

### Prerequisites

- macOS with Xcode installed
- Rust with iOS targets: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`
- Dioxus CLI: `cargo install dioxus-cli`

### Environment

```bash
cp .env.example .env
# Add your API keys (or configure later in-app via Settings)
SONIOX_API_KEY=your_key
OPENAI_API_KEY=your_key
ANTHROPIC_API_KEY=your_key   # optional, only if you pick Anthropic in Settings
```

- Soniox: https://console.soniox.com
- OpenAI: https://platform.openai.com/api-keys
- Anthropic: https://console.anthropic.com

API keys and LLM provider can also be set in-app (sidebar → Settings) — stored in SQLite, no recompile needed.
OpenAI key is always required (used for embeddings). Anthropic key is required only when Provider = Anthropic.

### Run

```bash
make dev      # iOS simulator
make ddev     # physical iPhone (USB or Wi-Fi)
make desktop  # macOS desktop (real mic)
make check    # cargo fmt + clippy
make clean    # remove dx caches (~2-3 GB)
make clean-all # cargo clean (full nuke)
```

### Tests

```bash
cargo test                    # 101 tests (unit + integration, includes attachment_test)
cargo test -- --ignored       # E2E tests (needs OPENAI_API_KEY and/or ANTHROPIC_API_KEY)
```

### Physical iPhone setup

1. Enable Developer Mode: Settings > Privacy & Security > Developer Mode
2. Connect via USB, trust the computer
3. Pair: `xcrun devicectl manage pair --device <ID>`
4. Create provisioning profile via temporary Xcode project (see CLAUDE.md)
5. After first pairing, Wi-Fi works on same network

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright 2026 Mirko Bozzetto

Licensed under the [European Union Public Licence (EUPL) v1.2](LICENSE).

You may use, modify and distribute this software under the terms of the EUPL.
Derivative works must remain open source under the EUPL or a [compatible licence](https://interoperable-europe.ec.europa.eu/collection/eupl/solution/eupl-compatible-open-source-licences) (GPL v3, MPL 2, LGPL, etc.).
