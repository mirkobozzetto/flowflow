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
    chat_surface.rs        chat/generic-action tool surface: which connector tools an open agent run mounts, under which contract (reads pass, writes hold for approval); the ONLY open-ended mount point
    tools/                 rig Tool trait (search, create, summarize) + multi-contract ContractHook (prefix-resolved, fail-closed)
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
    app/                   root + routing (mod, router, nav, top_bar, fab, boot, consent, watchers, restore_lock, animations, contexts)
    chat/                  ChatView + RAG chat (view, chat_input, actions, action_card, bubbles, sources_accordion, menu, typing_indicator, empty_state, models)
    notes/                 list + detail (note_list, note_card, detail/ + detail/hooks/, attachments, tags, reminders, folder_picker, audio_player, menu)
    sidebar/               drawer (mod, folders, conversations) + use_swipe_drawer
    thread/                folder-scoped thread chat (card, detail, entry_button, header_menu)
    recording/             recording bar + 60fps waveform (bar, controls, waveform)
    settings/              tabs (general, intelligence, transcription, connections, account, backup, privacy, storage, shortcuts)
    sync/                  pairing + conflicts UI (controls, pairing, conflicts)
    hooks/                 reusable hooks (swipe.rs = use_swipe_drawer / use_sheet_dismiss, +.ts source -> .js via `make js`)
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

Pinned versions and why: `docs/guides/stack.md`. Key constraints to keep in mind:
Dioxus 0.7, Rust 1.94.1, `IPHONEOS_DEPLOYMENT_TARGET=16.0` (required by
lancedb/zstd-sys), and arrow-array/arrow-schema must match lancedb's arrow version.

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
Soniox: <https://console.soniox.com> — OpenAI: <https://platform.openai.com/api-keys> — Anthropic: <https://console.anthropic.com>

## References

- `docs/guides/stack.md`: pinned dependency versions and why each is pinned
- `docs/guides/device-setup.md`: one-time iPhone setup, provisioning profile, manual simctl/devicectl commands
- `docs/guides/appstore.md`: App Store build and submission
- `docs/HISTORY.md`: what shipped, milestone by milestone, plus the original track board
- `ANALYSIS.md`: full SuperPowerNotes analysis
- `INSTRUCTIONS.md`: first session startup brief
- SuperPowerNotes source: `/Users/mirkobozzetto/stuffs/superpowernotes`

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **flowflow** (7949 symbols, 35703 relationships, 644 execution flows).

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact before editing.** Use `impact({target: "symbolName", direction: "upstream"})` or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .`; report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
- MUST warn on HIGH/CRITICAL `risk` pre-edit; never use `riskSharedAxes` to waive a HIGH/CRITICAL `risk` warning. Compare File/symbol: MCP File omits axes; Graph-RAG expands File.
- **MUST treat `risk: UNKNOWN` as unresolved, not as low.** An empty caller set is not evidence the symbol is unused — it can also mean the callers are not resolvable by the index (plain-object property access, dynamic dispatch, cross-language calls). `impact` pairs `UNKNOWN` with a `riskNote` saying so. Confirm with a text search before treating the symbol as safe to change or delete; do not proceed on the strength of a zero.
- **MUST use `query({search_query: "concept"})` for concepts/flows, `context({name: "symbolName"})` for a named symbol, or `impact` for blast radius, on read-only callers, dependencies, imports, or execution flow.** Graph first; text search only for empty/`UNKNOWN`/literals.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method before MCP/CLI impact analysis.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis, and never read `UNKNOWN` as an all-clear — it means the walk could not answer, which is the one verdict that requires confirming by other means.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit before MCP/CLI graph change analysis.

## Resources

| Resource | Use for |
| --- | --- |
| `gitnexus://repo/flowflow/context` | Codebase overview, check index freshness |
| `gitnexus://repo/flowflow/clusters` | All functional areas |
| `gitnexus://repo/flowflow/processes` | All execution flows |
| `gitnexus://repo/flowflow/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
| --- | --- |
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
