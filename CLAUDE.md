# CLAUDE.md — FlowFlow

## Project

**FlowFlow** — 100% Rust mobile app for recording voice notes, transcribing them, generating tags/titles via AI, organizing in folders, and chatting with notes via local RAG.

Inspired by SuperPowerNotes (Next.js/TypeScript app by Mirko). See `ANALYSIS.md` for full source codebase analysis.

## Owner

Mirko Bozzetto — freelance full-stack developer, Brussels.

## Technical Constraints

- **Language**: 100% Rust. Zero JavaScript, zero TypeScript.
- **UI**: Dioxus (native mobile support)
- **Target**: iOS only (Android later)
- **Vector DB**: LanceDB (local semantic search)
- **Metadata**: SQLite (rusqlite)
- **Transcription**: Soniox REST API (async)
- **Embeddings**: OpenAI API (text-embedding-3-small)
- **HTTP**: reqwest
- **Async**: tokio

## Work Methodology

- One step at a time. After each step → STOP → show result → wait for Mirko's validation.
- No predefined file structure beyond current step.
- If it doesn't compile or work → fix before moving forward.
- No assumptions — test everything.
- When unsure between two approaches → present options with pros/cons → Mirko decides.
- Git: commit after each validated step, descriptive messages.
- File names, structure, architecture evolve as needed. Nothing is set in stone.

## Tracks (indicative order, Mirko decides)

| Track | Description | Status |
|-------|-------------|--------|
| A | Minimal Dioxus iOS scaffold (hello world on simulator) | Done |
| B | Audio capture iOS mic (cpal + hound, save WAV) | Done |
| C | Soniox REST (upload WAV → transcription) | Done |
| D | SQLite storage + UI refactor + Tailwind | Done |
| E | Embeddings + RAG + Chat | In progress |

### Track E Progress

| Step | Description | Status |
|------|-------------|--------|
| 1 | OpenAI client (embed + chat + chunking) | Done |
| 2 | LanceDB VectorStore (store, search, delete) | Done |
| 3 | Auto-embed on note save (validated on iPhone) | Done |
| 4 | Settings UI for API keys (in-app) | — |
| 5 | Tags UI on NoteDetail (chips, add/remove, LLM auto-gen) | — |
| 6 | Chat UI + RAG pipeline (search → context → response) | Done |

## Architecture (Clean Architecture, SRP — 31 modules)

```
src/
  main.rs                entry point (dotenvy + dioxus::launch)
  models/                domain entities
    note.rs              Note, NewTextNote, UpdateNote, NoteType
    folder.rs            Folder, NewFolder, UpdateFolder
  db/                    persistence layer
    mod.rs               Database struct, open, conn, migrate
    schema.rs            SQL schemas, MIGRATIONS const
    note_repo.rs         CRUD notes
    folder_repo.rs       CRUD folders
  services/              business logic
    constants.rs         AI config (models, dims, chunks, RAG prompt, top_k)
    ai.rs                OpenAIClient (embed, chat), chunk_text
    vectordb.rs          VectorStore (LanceDB: store, search, delete)
    embed.rs             embed_note, delete_note_embeddings (background)
    rag.rs               RAG pipeline (embed query → vector search → context → LLM)
    audio.rs             AudioRecorder (cpal, WAV capture)
    transcription.rs     SonioxClient (upload, poll, transcribe)
  platform/              OS-specific
    ios.rs               AVAudioSession, documents_dir
  ui/                    Dioxus components (1 component = 1 file)
    mod.rs               App root component + view routing
    state.rs             AppState, View enum (NotesList, NoteDetail, Chat)
    top_bar.rs           TopBar (navigation, back button, chat icon)
    sidebar.rs           SidebarOverlay, FolderSection, FolderItem
    fab.rs               FloatingActionButton (new note)
    note_list.rs         NotesList
    note_card.rs         NoteCard
    note_detail.rs       NoteDetail (orchestrator, auto-embed on save)
    folder_picker.rs     FolderPicker (dropdown)
    recording_bar.rs     RecordingBar (voice recording in notes)
    chat.rs              ChatView (RAG chat orchestrator, messages, typing indicator)
    chat_input.rs        ChatInputBar (mic, text input, send, transcription)
    icons.rs             Phosphor SVG icons (ChatAi, HeadCircuit, PaperPlaneRight, etc.)
```

## Data Entities

### Note (global entity — voice and text are input modes, not types)
- id (UUID), note_type (voice/text), title, content, tags[], duration, audio_file_path
- Auto-titled with date+time when created on the fly
- Speech-to-text available from any note
- Embedding: auto-embedded on save (>50 chars) → chunked → OpenAI → LanceDB

### Folder (hierarchy)
- id, name, description, parent_id (self-ref), created_at
- N:N relation with Note via junction table (notes_folders)
- ON DELETE SET NULL for parent (children become root)

## Styling

- **Tailwind CSS V4** via Dioxus 0.7 auto-detection
- `tailwind.css` at project root = input file
- `dx serve` auto-compiles to `assets/tailwind.css`
- Custom colors: `ios-green` (#34c759), `ios-red` (#ff3b30), `ios-blue` (#007aff)
- Mobile-first: touch targets 44px, safe area insets, scroll fix on empty pages
- Animations: slideInRight/slideOutRight (views), fadeInUp (chat messages), typingDot (loading), pulseSoft (transcription)

## Main Pipelines

```
Note Pipeline:
  Mic capture → WAV → Soniox REST → transcription
    → SQLite (metadata) + auto-embed on save
    → OpenAI embed → chunk → LanceDB (vector)

RAG Chat Pipeline:
  User question → OpenAI embed (query vector)
    → LanceDB vector search (top 5 chunks)
    → Build context from matched notes
    → OpenAI chat (system prompt + context + question)
    → Response with source citations
```

## Stack Versions

- Dioxus 0.7 (CLI dx 0.7.7)
- cpal 0.17 (audio I/O via CoreAudio on iOS)
- hound 3.5 (WAV file writing)
- reqwest 0.12 (HTTP client, multipart + JSON)
- tokio 1 (async runtime)
- serde 1.0 + serde_json 1.0 (JSON serialization)
- dotenvy 0.15 (.env loader)
- rusqlite 0.34 (SQLite, bundled for iOS cross-compile)
- uuid 1 (UUID v4 generation)
- chrono 0.4 (ISO 8601 timestamps)
- Tailwind CSS V4 (auto-detected by dx)
- lancedb 0.27.2 (vector DB, default-features = false for iOS)
- arrow-array 57 + arrow-schema 57 (must match lancedb's arrow version)
- futures 0.3.32 (stream collect for LanceDB queries)
- Rust 1.94.1
- iOS targets: aarch64-apple-ios, aarch64-apple-ios-sim
- IPHONEOS_DEPLOYMENT_TARGET=16.0 (required for lancedb/zstd-sys)

## Commands (use Makefile)

```bash
make build    # cargo build --features mobile
make format   # cargo fmt
make check    # fmt check + clippy
make dev      # dx serve --ios (simulator, IPHONEOS_DEPLOYMENT_TARGET=16.0)
make ddev     # dx serve --ios --device (physical iPhone, WiFi OK)
make desktop  # dx serve --desktop (Mac window, real mic)
make logs     # open Console.app (select iPhone, filter "FlowFlow")
```

## Environment Variables

Create a `.env` file at the project root (never committed):

```
SONIOX_API_KEY=your_soniox_api_key
OPENAI_API_KEY=your_openai_api_key
```

Keys are captured at compile time via `option_env!()` (iOS has no runtime env vars).
Soniox: https://console.soniox.com — OpenAI: https://platform.openai.com/api-keys

### Manual commands (if needed)

```bash
# Simulator management
open /Applications/Xcode.app/Contents/Developer/Applications/Simulator.app
xcrun simctl boot "iPhone 17 Pro"
xcrun simctl shutdown all

# Device management
xcrun devicectl list devices
xcrun devicectl manage pair --device <DEVICE_ID>
```

## Physical Device Setup (one-time)

1. iPhone: Settings → Privacy & Security → Developer Mode → enable → restart
2. Connect via USB, accept "Trust This Computer"
3. `xcrun devicectl manage pair --device <ID>` (fixes the "no DDI" error)
4. Xcode → Settings → Apple Accounts → click account → Manage Certificates → + → Apple Development
5. If certificate not recognized by codesigning, install Apple WWDR intermediate cert:
   `curl -sO https://www.apple.com/certificateauthority/AppleWWDRCAG3.cer && security add-certificates AppleWWDRCAG3.cer && rm AppleWWDRCAG3.cer`
6. Verify: `security find-identity -v -p codesigning` must show "Apple Development"
7. Create provisioning profile (required for free Apple account):
   WORKAROUND: this method is cumbersome. Looking for a simpler approach.
   No existing Dioxus issue on this — worth opening one if dx doesn't improve this.
   Ref: https://github.com/DioxusLabs/dioxus/issues/3817 (related App Store issue)
   For now, create a TEMPORARY Swift/SwiftUI project in Xcode:
   - Xcode → File → New → Project → iOS → App
   - Product Name: `flowflow`, Organization Identifier: `com.mirkobozzetto`, Team: Personal Team
   - Interface: SwiftUI, Language: Swift (doesn't matter, it's temporary)
   - Save to /tmp
   - Select iPhone as destination at the top of Xcode
   - Cmd+R to build — Xcode creates the provisioning profile automatically
   - Trust the dev profile on iPhone: Settings → General → VPN & Device Management → Trust
   - Close the Xcode project (profile stays in ~/Library/Developer/Xcode/UserData/Provisioning Profiles/)
   The profile is tied to the bundle ID (com.mirkobozzetto.flowflow), not the language.
   Once created, delete the Xcode project — the profile persists and `dx serve` uses it for the Rust app.
8. After first pairing, Wi-Fi works (same network)

## References

- `ANALYSIS.md`: full SuperPowerNotes analysis
- `INSTRUCTIONS.md`: first session startup brief
- SuperPowerNotes source: `/Users/mirkobozzetto/stuffs/superpowernotes`

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **flowflow** (606 symbols, 964 relationships, 26 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/flowflow/context` | Codebase overview, check index freshness |
| `gitnexus://repo/flowflow/clusters` | All functional areas |
| `gitnexus://repo/flowflow/processes` | All execution flows |
| `gitnexus://repo/flowflow/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
