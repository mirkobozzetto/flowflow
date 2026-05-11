# FlowFlow

Mobile voice notes app with AI chat — 100% Rust, Dioxus iOS, local-first (SQLite + LanceDB).

Built with Dioxus 0.7 for iOS. Inspired by [SuperPowerNotes](https://github.com/mirkobozzetto/superpowernotes).

## Features

- **Unified note editor** — text + voice in a single view, auto-save on exit via `use_drop`
- **Voice dictation** — tap to record, real-time audio visualization (12-bar waveform, 80ms polling)
- **Transcription** — Soniox REST API (stt-async-v4, French-optimized)
- **Auto-embed** — notes auto-embedded on save (>50 chars) → chunked → OpenAI → LanceDB
- **AI tags** — LLM auto-generated tags (3-5 per note) + manual add/remove chips
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
- **SQLite storage** — local-first with WAL mode, foreign keys, versioned migrations (V1 + V2)
- **Tailwind CSS V4** — utility-first styling via Dioxus 0.7 auto-detection

## Stack

| Layer | Tech |
|-------|------|
| Language | Rust 1.94 |
| UI Framework | Dioxus 0.7 (WKWebView on iOS) |
| Styling | Tailwind CSS V4 |
| Audio | cpal 0.17 + hound 3.5 |
| Transcription | Soniox REST API (stt-async-v4) |
| Database | SQLite (rusqlite 0.34, bundled) |
| Vector DB | LanceDB 0.27.2 (local semantic search) |
| LLM Framework | rig-core 0.36 (OpenAI provider) |
| Embeddings | OpenAI API (text-embedding-3-small, 1536 dims) |
| Chat | OpenAI API (gpt-4o-mini) via rig agent |
| Markdown | pulldown-cmark 0.12 |
| HTTP | reqwest 0.13 (multipart + JSON + rustls) |
| Async | tokio 1 + futures-timer 3 |
| Serialization | serde 1.0 + serde_json 1.0 |
| Date/Time | chrono 0.4 |
| UUID | uuid 1 (v4) |
| Env | dotenvy 0.15 |
| iOS Platform | objc2 + objc2-avf-audio |
| Icons | Phosphor Icons (MIT) |
| Target | iOS (aarch64-apple-ios, aarch64-apple-ios-sim) |

## Architecture

```
src/                          38 modules, ~5000 lines Rust
  main.rs                     entry point (dotenv, AVAudioSession, launch)
  lib.rs                      pub mod exports (enables integration tests)
  models/
    note.rs                   Note, NoteType (FromStr), NewTextNote, UpdateNote
    folder.rs                 Folder, NewFolder, UpdateFolder
    conversation.rs           Conversation, ConversationMessage
  db/
    mod.rs                    Database struct, migrations (V1 + V2), db_path()
    schema.rs                 SQL schemas (notes, folders, settings, conversations)
    note_repo.rs              CRUD notes
    folder_repo.rs            CRUD folders
    settings_repo.rs          get/set settings (key-value store)
    conversation_repo.rs      CRUD conversations + messages
  services/
    constants.rs              AI config (models, dims, prompts: RAG, tags, agent, summarize)
    ai.rs                     chunk_text (text splitting with overlap)
    llm.rs                    LlmClient (rig-core wrapper: embed, chat, generate_tags)
    error.rs                  LlmError enum (NotConfigured, Embedding, Completion, TagParsing)
    tools.rs                  Agent tools: SearchNotes, CreateNote, SummarizeFolder (rig Tool trait)
    vectordb.rs               VectorStore (LanceDB: store, search, delete, cosine)
    embed.rs                  embed_note, delete_note_embeddings (background thread)
    rag.rs                    RAG pipeline (embed → search → context → agent with tools)
    audio.rs                  AudioRecorder (cpal stream, samples, levels, duration)
    transcription.rs          SonioxClient (upload, poll, transcribe)
  platform/
    ios.rs                    AVAudioSession, documents_dir
  ui/
    mod.rs                    App component, view routing, keyboard handler
    state.rs                  AppState (11 signals), View enum (4 variants)
    top_bar.rs                TopBar (contextual back, chat icon)
    sidebar.rs                Tabs (Notes/Chats), ConversationItem, FolderItem
    fab.rs                    FloatingActionButton
    note_list.rs              NotesList (filtered by folder)
    note_card.rs              NoteCard (preview, tags, folder badge)
    note_detail.rs            NoteDetail (tags chips, auto-tag, recording, auto-save)
    folder_picker.rs          FolderPicker (dropdown)
    recording_bar.rs          RecordingBar (audio visualization + timer)
    settings.rs               SettingsView (API keys form)
    chat.rs                   ChatView (persistent conversations, markdown, sources)
    chat_input.rs             ChatInputBar (mic, textarea, send, transcription)
    icons.rs                  18 Phosphor SVG icon components
```

## Data Model

### SQLite Schema (V1 + V2 migrations)

```
notes (id, note_type, title, content, audio_file_path, duration_secs, tags[], created_at, modified_at)
folders (id, name, description, parent_id → self-ref, created_at, modified_at)
notes_folders (folder_id, note_id) — N:N junction, CASCADE on delete
settings (key PK, value) — API keys stored locally
conversations (id, title, created_at, modified_at)
conversation_messages (id, conversation_id → CASCADE, role, content, sources_json, created_at)
```

### Pipelines

```
Note Pipeline:
  Mic → cpal → WAV → Soniox REST → transcription
    → SQLite + auto-embed on save
    → chunked → OpenAI embed → LanceDB (cosine)

RAG Chat Pipeline:
  User question → OpenAI embed (query vector)
    → LanceDB search (top 5 chunks, cosine)
    → Build context from matched notes
    → rig Agent with tools (search_notes, create_note, summarize_folder)
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
```

- Soniox: https://console.soniox.com
- OpenAI: https://platform.openai.com/api-keys

API keys can also be set in-app (sidebar → Settings) — stored in SQLite, no recompile needed.

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
cargo test                    # 65 tests (unit + integration)
cargo test -- --ignored       # E2E tests (needs OPENAI_API_KEY)
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
- [x] Track B — Audio capture (cpal + hound, WAV recording)
- [x] Track C — Soniox transcription (French, stt-async-v4)
- [x] Track D — SQLite + folders + UI refactor + Tailwind
- [x] Track E — Embeddings + RAG + Chat + Settings + Tags
- [x] Track F Step 1 — RIG framework migration (LlmClient, LlmError, 40 tests)
- [x] Track F Step 2 — Agent tools (SearchNotes, CreateNote, SummarizeFolder), reqwest 0.13
- [x] Unified note editor (text + inline voice dictation)
- [x] Auto-save on exit (use_drop hook)
- [x] Auto-embed notes on save (chunked → OpenAI → LanceDB)
- [x] AI auto-tagging (LLM generates 3-5 tags per note)
- [x] RAG chat with markdown rendering and source cards
- [x] Chat history persistence (SQLite, sidebar tabs)
- [x] In-app API key settings (no .env recompile)
- [x] iOS keyboard handling (visualViewport API)
- [x] Contextual back navigation (Chat → Note → back to Chat)
- [x] Conversation management (rename, delete, same UX as folders)

- [x] PromptHook — tool call status in chat UI (rig PromptHook trait, mpsc channel)
- [x] Note search bar (live filter by title/content/tags)
- [x] make clean / make clean-all targets

### Next

- [ ] Track F Step 3 — Multi-provider LLM (Anthropic via rig)
- [ ] Settings UI for LLM provider selection + Anthropic API key
- [ ] Document import (PDF, TXT, DOC)
- [ ] Full-text search (SQLite FTS5) — hybrid with semantic search

## License

Private project by Mirko Bozzetto.
