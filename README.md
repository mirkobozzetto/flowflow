# FlowFlow

100% Rust mobile app for voice notes with transcription, folder organization, and AI features.

Built with Dioxus 0.7 for iOS. Inspired by [SuperPowerNotes](https://github.com/mirkobozzetto/superpowernotes).

## Features

- **Unified note editor** — text + voice in a single view, auto-save on exit via `use_drop`
- **Voice dictation** — tap to record, real-time audio visualization (12-bar voice-reactive waveform via RMS analysis)
- **Transcription** — Soniox REST API (stt-async-v4, French-optimized with `language_hints_strict`)
- **Auto-append transcription** — dictated text appends to note content automatically
- **French auto-title** — notes auto-titled with local date ("8 mai 2026, 14:30")
- **Folder management** — create, rename, delete folders and subfolders from sidebar drawer
- **Folder assignment** — inline picker in note editor, auto-assign from folder context
- **Note deletion** — with `deleted` flag to prevent auto-save re-creation
- **Slide transitions** — iOS-style push/pop animations (150ms, CSS keyframes)
- **Phosphor + Josemi icons** — 14 SVG icon components (monochrome gray/blue/white palette)
- **iOS keyboard toolbar hidden** — objc2 method swizzle on WKContentView `inputAccessoryView`
- **SQLite storage** — local-first with WAL mode, foreign keys, versioned migrations
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
| HTTP | reqwest 0.12 (multipart + JSON) |
| Async | tokio 1 + futures-timer 3 |
| Serialization | serde 1.0 + serde_json 1.0 |
| Date/Time | chrono 0.4 |
| UUID | uuid 1 (v4) |
| Env | dotenvy 0.15 |
| iOS Platform | objc2 + objc2-avf-audio |
| Icons | Phosphor Icons + Josemi Icons (MIT) |
| Target | iOS (aarch64-apple-ios, aarch64-apple-ios-sim) |

## Architecture

```
src/                          16 files, ~2100 lines Rust
  main.rs                     entry point (dotenv, AVAudioSession, keyboard hack, launch)
  models/
    mod.rs                    re-exports
    note.rs                   Note, NoteType, NewVoiceNote, NewTextNote, UpdateNote, generate_auto_title()
    folder.rs                 Folder, NewFolder, UpdateFolder
  db/
    mod.rs                    Database struct, migrations (V1 schema), db_path(), now_iso()
    note_repo.rs              CRUD notes (create voice/text, get, list, list_in_folder, update, update_audio_metadata, delete)
    folder_repo.rs            CRUD folders (create, get, list_all, list_root, list_subfolders, update, delete, add/remove_note, folders_for_note)
  services/
    mod.rs                    re-exports
    audio.rs                  AudioRecorder (cpal stream, samples buffer, start/stop/current_levels/duration), output_dir(), WAV writing
    transcription.rs          SonioxClient (upload_file, create_transcription, poll_transcript, fetch_transcript, transcribe)
  platform/
    mod.rs                    re-exports
    ios.rs                    configure_audio_session(), hide_keyboard_accessory(), documents_dir()
  ui/
    mod.rs                    App component, AppState (8 signals), View enum, slide transition overlay
    icons.rs                  14 SVG components (IconNewNote, IconMic, IconStop, IconTrash, IconPencil, IconFolderPlus, IconFolder, IconDotsThree, IconPlus, IconArrowLeft, IconList, IconCheck, IconX, IconFloppyDisk)
    layout.rs                 TopBar (back/menu, folder name), SidebarOverlay (drawer), FolderSection (create), FolderItem (rename/subfolder/delete/expand), FloatingActionButton (80px)
    notes.rs                  NotesList (filtered by folder), NoteCard (preview, folder badge), NoteDetail (editor, folder picker, inline recording, auto-save, delete)
```

## Data Model

### SQLite Schema (V1 migration)

```
notes (id, note_type, title, content, audio_file_path, duration_secs, tags[], created_at, modified_at)
folders (id, name, description, parent_id → self-ref, created_at, modified_at)
notes_folders (folder_id, note_id) — N:N junction, CASCADE on delete
```

- `parent_id` ON DELETE SET NULL (orphaned children become root)
- WAL journal mode, foreign keys enabled
- Tags stored as JSON array

### Execution Flows (GitNexus, 13 processes)

| Flow | Path |
|------|------|
| App startup | `main → dotenv → AVAudioSession → hide_keyboard → launch(App)` |
| DB init | `App → Database.open → db_path → documents_dir → migrate` |
| Recording | `NoteDetail → AudioRecorder.start → cpal stream → samples buffer` |
| Stop + transcribe | `AudioRecorder.stop → write_wav → start_transcription → SonioxClient.transcribe` |
| Transcription pipeline | `upload_file → create_transcription → poll_transcript → fetch_transcript` |
| Auto-save | `NoteDetail unmount (use_drop) → create_text_note / update_note + folder assignment` |
| Folder tree | `FolderSection → list_root_folders → FolderItem → list_subfolders (recursive)` |
| Note listing | `NotesList → list_notes / list_notes_in_folder (reactive via notes_version)` |

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
- [x] Track B — Audio capture (cpal + hound, WAV recording)
- [x] Track C — Soniox transcription (French, batch quality, stt-async-v4)
- [x] Track D — SQLite + folders + UI refactor + Tailwind
- [x] Unified note editor (text + inline voice dictation)
- [x] Auto-save on exit (use_drop hook)
- [x] Auto-append transcription to note content after dictation
- [x] French auto-title (date format: "8 mai 2026, 14:30")
- [x] Folder management (create/rename/delete/subfolders from drawer)
- [x] Folder assignment + picker in note editor
- [x] Note deletion (with deleted flag protection)
- [x] Slide transitions (push/pop iOS style, 150ms)
- [x] Phosphor + Josemi icon system (14 SVG components)
- [x] Voice-reactive audio visualization (12 bars, real-time RMS, 80ms polling)
- [x] iOS keyboard toolbar hidden (objc2 swizzle on WKContentView)
- [x] Monochrome design system (gray/blue/white)
- [x] iOS AVAudioSession (PlayAndRecord category)
- [x] Platform-specific paths (iOS Documents dir vs temp)

### Next

- [ ] Search notes (full-text search on title + content, SQLite FTS5)
- [ ] Track E — Embeddings (OpenAI text-embedding-3-small + LanceDB) for semantic search
- [ ] Track F — RAG + Chat (embed > search > context > LLM)
- [ ] AI-generated titles and tags
- [ ] Dark mode (monochrome palette ready)
- [ ] Export notes (PDF, markdown)

## License

Private project by Mirko Bozzetto.
