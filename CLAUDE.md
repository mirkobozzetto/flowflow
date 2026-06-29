# CLAUDE.md — FlowFlow

## Project

**FlowFlow** — 100% Rust mobile app for recording voice notes, transcribing them, generating tags/titles via AI, organizing in folders, and chatting with notes via local RAG.

Inspired by SuperPowerNotes (Next.js/TypeScript app by Mirko). See `ANALYSIS.md` for full source codebase analysis.

## Owner

Mirko Bozzetto — freelance full-stack developer, Brussels.

## Technical Constraints

- **Language**: principalement Rust. JS/TS toléré seulement comme glue impérative minimale dans la webview (`document::eval`: canvas, scroll, clipboard, keyboard) et FFI natif (objc2/Swift) pour les primitives plateforme. Pas de framework JS, pas de logique métier hors Rust.
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
- Builds and installs are run by Claude, not Mirko. Claude runs the toolchain itself (`cargo`, `make all` for the iOS device, `make desktop-app` for the Mac app), fixes what breaks, and then hands Mirko only the manual test steps (what to tap and observe). Never hand Mirko a build or serve command (`make ddev`/`make dev`/etc.) to run. Mirko validates on device; Claude does not push until he has.
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
| E | Embeddings + RAG + Chat + Settings + Tags | Done |
| F | RIG framework + agent tools + multi-provider | Done |
| G | Document attachments (TXT, MD, CSV, PDF, DOCX) | Done |
| H | Pluggable STT providers + local Whisper models (RFC 0005) | Built - device validation pending |

### Track F Progress

| Step | Description | Status |
|------|-------------|--------|
| 1 | RIG migration (LlmClient replaces OpenAIClient) | Done |
| 2 | Agent tools + reqwest unification | Done |
| 3 | Multi-provider (Anthropic) | Done |

### Track F — Future

- Mistral provider via rig-core abstractions
- Additional agent tools (search by date, link notes, batch tag generation)
- Prerequisite: validate iOS cross-compilation on each new tool

### Track G Progress

| Step | Description | Status |
|------|-------------|--------|
| 1 | SQLite V3 migration + Attachment model + repo | Done |
| 2 | Attachment cards UI in NoteDetail + modal viewer | Done |
| 3 | Native iOS file picker (UIDocumentPickerViewController) | Done |
| 4 | PDF parsing via pdf-extract | Done |
| 5 | DOCX parsing via zip + quick-xml | Done |
| 6 | Auto-embed attachments on import (chunked, scheme `att:{id}:{idx}`) | Done |
| 7 | Tests (migration V3, CRUD, cascade delete, DOCX, PDF crate) | Done |

### Track E Progress

| Step | Description | Status |
|------|-------------|--------|
| 1 | OpenAI client (embed + chat + chunking) | Done |
| 2 | LanceDB VectorStore (store, search, delete) | Done |
| 3 | Auto-embed on note save (validated on iPhone) | Done |
| 4 | Settings UI for API keys (in-app, SQLite) | Done |
| 5 | Tags UI on NoteDetail (chips, add/remove, LLM auto-gen) | Done |
| 6 | Chat UI + RAG pipeline (search → context → response) | Done |
| 7 | Chat history persistence (SQLite, sidebar tabs, CRUD) | Done |

## Architecture (Clean Architecture, SRP — 202 modules, 4 layers)

Layered clean architecture. Dependencies point inward only:
`ui` / `infrastructure` -> `application` -> `domain`. `domain` imports nothing
outward; a view never orchestrates (it delegates to `application`).

LlmClient dispatches on `Provider` enum: OpenAI (embeddings + chat/agent) or Anthropic (chat/agent only, embeddings always OpenAI). Provider, OpenAI key, and Anthropic key persisted in SQLite via Settings UI.

```
src/
  main.rs                  entry point (dotenvy + dioxus::launch)
  lib.rs                   pub mod exports (enables integration tests in tests/)
  prelude.rs               shared re-exports

  domain/                  entities + pure rules (no IO, no outward deps)
    note.rs, folder.rs, thread.rs, conversation.rs, attachment.rs, reminder.rs
    agent_manifest.rs, governance.rs, orchestration.rs   agent/marketplace contracts

  application/             use cases + orchestration (composes domain + infra)
    constants.rs           AI config + prompts (RAG_AGENT_SYSTEM_PROMPT, RAG_AGENT_WEB_SYSTEM_PROMPT, NOTE_ACTION_PROMPT, SUMMARIZE_FOLDER_PROMPT, tags)
    rag/                   RAG pipeline: mod.rs (query: scope gate -> hybrid search -> context -> agent), fusion (RRF), temporal, scoring, rerank, config, context
    web_search.rs          Exa web search (exa_search) merged via RRF when web_on
    tools/                 rig Tool trait (search, create, summarize) + prompt_agent_with_tools
    embed/                 embed_note/attachment + chunk_store (background)
    transcription_manager/ STT job orchestration (job, append, processing)
    backup/                backup/export/restore (archive, snapshot, stage, swap, validate, manifest, paths)
    tagging.rs, titling.rs, intent.rs, reminders.rs, chain.rs, agent_builder.rs, connector_module.rs, note_persistence.rs, ai.rs, i18n/, error.rs

  infrastructure/          IO: SQLite, LLM, MCP, vector DB, sync, platform
    llm.rs                 LlmClient + Provider enum (embed, chat, run_mcp_agent generic primitive)
    vectordb.rs            VectorStore (LanceDB: hybrid_search, store, delete, fts index)
    persistence/           Database + repos (note, folder, thread, conversation, attachment, settings, peer, chunk, reminder, schema/migrations)
    mcp/                   MCP client
    backend/               backend HTTP client (accounts, entitlements, signed agent fetch)
    transcription/         Soniox cloud + local Whisper (client, provider, whisper, models, hesitations)
    sync/                  P2P sync: engine, protocol/{collect,apply/,session,wire,catalog}, peers/{lan,codec,account_join,identity,peer_store,host,join}, vv, reconcile, transport, conflict, gc
    platform/              iOS/macOS FFI (ios/{picker,player,share,reminders,live_activity,sync_ffi}, parsers, pdf)
    audio.rs               AudioRecorder (cpal, WAV capture)

  ui/                      Dioxus components (1 component = 1 file)
    app/                   root + routing (mod, router, nav, top_bar, fab, boot, consent, watchers, right_nav, restore_lock, animations, contexts)
    chat/                  ChatView + RAG chat (view, chat_input, actions, action_card, bubbles, sources_accordion, menu, typing_indicator, empty_state, models)
    notes/                 list + detail (note_list, note_card, detail/ + detail/hooks/, attachments, tags, reminders, folder_picker, audio_player, menu)
    sidebar/               drawer (mod, folders, conversations) + use_swipe_drawer
    thread/                folder-scoped thread chat (card, detail, entry_button, header_menu)
    recording/             recording bar + 60fps waveform (bar, controls, waveform)
    settings/              tabs (general, intelligence, transcription, connections, account, backup, privacy, storage, shortcuts)
    sync/                  pairing + conflicts UI (controls, pairing, conflicts)
    hooks/                 reusable hooks (swipe.rs = use_swipe_drawer / use_swipe_right_nav, +.ts source -> .js via `make js`)
    keyboard/, state.rs, kit.rs, icons.rs, clipboard.rs
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

### Conversation (chat history)
- id (UUID), title (auto from first question, 50 chars), created_at, modified_at
- Messages: id, conversation_id (FK CASCADE), role (user/bot), content, sources_json
- Sidebar tabs Notes/Chats with rename/delete (same UX as folders)

### Attachment (imported document linked to a note)
- id (UUID), note_id (FK CASCADE on parent note), filename, content_text, imported_at
- Stored in SQLite (V3 migration), one-to-many with Note via `attachments.note_id`
- Auto-embedded on import (>50 chars) → chunked → OpenAI → LanceDB
  - Vector chunk id scheme: `att:{attachment_id}:{chunk_idx}` (distinct from note chunks)
- Supported formats: TXT, MD, CSV (direct), PDF (pdf-extract), DOCX (zip + quick-xml)
- CASCADE delete on parent note removal

### Settings (key-value)
- key (PK), value — stores API keys and `llm_provider` in SQLite
- Known keys: `openai_api_key`, `anthropic_api_key`, `soniox_api_key`, `llm_provider` (openai/anthropic), `stt_provider` (soniox/whisper_local), `whisper_model` (catalog id)
- Fallback chain: DB → env var → compile-time `option_env!()`
- `stt_provider`/`whisper_model` travel in backup; model files never do (device-local artifacts)

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
  Mic capture → WAV → TranscriptionClient (Soniox REST or local Whisper per stt_provider)
    → SQLite (metadata) + auto-embed on save
    → OpenAI embed → chunk → LanceDB (vector)

Local Whisper Path (offline):
  Settings → download ggml model (size confirm, sha256 verified, .part+rename)
    → WhisperLocal: WAV decode → mono 16 kHz → whisper-rs (Metal) → clean_hesitations
    → airplane mode works end to end; consent gate applies to all providers

RAG Chat Pipeline:
  User question → OpenAI embed (query vector, always OpenAI)
    → LanceDB vector search (top 5 chunks)
    → Build context from matched notes
    → Agent with tools (search_notes, create_note, summarize_folder)
    → OpenAI or Anthropic chat per Provider setting (system prompt + context + question, up to 4 tool turns)
    → Response with source citations
```

## Stack Versions

- Dioxus 0.7 (CLI dx 0.7.7)
- cpal 0.17 (audio I/O via CoreAudio on iOS)
- hound 3.5 (WAV file writing)
- reqwest 0.13 (HTTP client, multipart + JSON, unified with rig-core)
- rig-core 0.36 (LLM abstraction, agent + tools, rustls feature, OpenAI + Anthropic providers)
- Anthropic provider (Claude Sonnet 4.6 default, max_tokens 4096, chat + agent + tools)
- tokio 1 (async runtime)
- serde 1.0 + serde_json 1.0 (JSON serialization)
- dotenvy 0.15 (.env loader)
- rusqlite 0.34 (SQLite, bundled for iOS cross-compile)
- uuid 1 (UUID v4 generation)
- chrono 0.4 (ISO 8601 timestamps)
- Tailwind CSS V4 (auto-detected by dx)
- lancedb 0.27.2 (vector DB, default-features = false for iOS)
- arrow-array 57 + arrow-schema 57 (must match lancedb's arrow version)
- pulldown-cmark 0.12 (markdown → HTML for chat responses)
- pdf-extract 0.10 (PDF text extraction)
- zip 2 (DOCX archive reading)
- quick-xml 0.36 (DOCX word/document.xml parser)
- futures 0.3.32 (stream collect for LanceDB queries)
- whisper-rs 0.16 (local STT, whisper.cpp via cmake, metal feature on apple targets)
- fs4 0.13 (free disk space check for model downloads)
- libc 0.2 (getrusage peak RSS in the whisper bench)
- Rust 1.94.1
- iOS targets: aarch64-apple-ios, aarch64-apple-ios-sim
- IPHONEOS_DEPLOYMENT_TARGET=16.0 (required for lancedb/zstd-sys)

## Commands (use Makefile)

```bash
make all            # build + sign widget + icon + install (auto-renews expired profiles)
make ddev           # dx serve --ios --device (hot reload, no widget signing)
make dev            # dx serve --ios (simulator)
make desktop        # dx serve --desktop (Mac window, real mic)
make build          # cargo build --features mobile
make format         # cargo fmt
make check          # fmt check + clippy
make deploy         # dx build device + icon injection
make appstore       # release build + distribution signing + IPA
make renew          # regenerate iOS provisioning profiles (xcodebuild + tools/provision-renew template)
make check-profiles # show profile expiration dates
make logs           # open Console.app (select iPhone, filter "FlowFlow")
make clean          # rm target/dx
```

## Environment Variables

Create a `.env` file at the project root (never committed):

```
SONIOX_API_KEY=your_soniox_api_key
OPENAI_API_KEY=your_openai_api_key
ANTHROPIC_API_KEY=your_anthropic_api_key
```

Keys can be set in-app (Settings view → saved in SQLite) or via `.env` at compile time.
Fallback chain: SQLite settings → env var → `option_env!()` compile-time.
OpenAI key is always required (used for embeddings). Anthropic key is required only when Provider is set to Anthropic in Settings.
Soniox: https://console.soniox.com — OpenAI: https://platform.openai.com/api-keys — Anthropic: https://console.anthropic.com

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

This project is indexed by GitNexus as **flowflow** (4302 symbols, 9968 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> Index stale? Run `node .gitnexus/run.cjs analyze` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? `npx gitnexus analyze` (npm 11 crash → `npm i -g gitnexus`; #1939).

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows. For regression review, compare against the default branch: `detect_changes({scope: "compare", base_ref: "main"})`.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit changes without running `detect_changes()` to check affected scope.

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
