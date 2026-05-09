# 07 — Vision

Product direction for FlowFlow. Inspired by Obsidian's knowledge management philosophy, adapted for mobile voice-first use.

## One-Line Pitch

**FlowFlow: your thoughts, transcribed and organized on your iPhone. Intelligence stays on-device.**

## Three Differentiators

### 1. Global Notes (No Voice/Text Dichotomy)
Every competitor separates "voice notes" from "text notes" (AudioPen, Otter, Apple Notes). FlowFlow treats a note as a note — the input mode (mic or keyboard) is an implementation detail, not a category.

- Speech-to-text available from any note, at any time
- Text and transcription live in the same content field
- The `note_type` field exists internally but never surfaces in the UI
- A note can start as text, gain a voice recording, get AI cleanup, and receive photo attachments — it's still one note

### 2. Local-First Intelligence (RAG Without Cloud)
Reor does this on desktop (Electron). Atomic does it on iOS (but cloud-dependent). Nobody does **mobile + local RAG + privacy** well.

- Embeddings via OpenAI API (text-embedding-3-small) — the only cloud dependency
- Vectors stored locally in LanceDB (file-based, no server)
- Semantic search runs entirely on device against local vectors
- Chat/RAG: retrieve local chunks → send to LLM with context
- Future: on-device embeddings via ONNX to eliminate even the embedding API call

Privacy positioning: "Your notes stay on your phone. Even search is local."

### 3. French-First Experience
The market is English-dominated. Very few apps optimize for French:
- Auto-titles in natural French ("Réunion budget Q3" not "Budget Meeting Q3")
- Soniox stt-async-v4 with `language_hints_strict: true` for French
- Date formatting: "8 mai 2026, 14:30" (not "May 8, 2026")
- Transcript cleanup preserving French grammar and idioms
- AI tags in French

This is a niche differentiator for francophone users (Belgium, France, Switzerland, Quebec, Africa).

## Obsidian-Inspired Features

### Backlinks Between Notes
- Syntax: `[[note title]]` or `[[note-id|display text]]`
- Auto-detect in content on save
- Backlinks panel in NoteDetail: "X notes link to this one"
- Tap backlink → navigate to linked note
- Implementation: parse `[[...]]` regex on save, store in `note_links (source_id, target_id)` junction table

### Tags as First-Class Citizens
- Inline `#tag` syntax (Bear-style)
- Auto-extracted from content on save
- Tag cloud in sidebar drawer (below folders)
- Filter notes by tag (within folder or globally)
- Tags inside folders: "Show me all #meeting notes in the Work folder"
- Nested tags: `#work/clients` (aspirational, not MVP)
- AI auto-generated tags after transcription

### Search by Criteria
- Full-text search (FTS5) for keyword matching
- Semantic search (LanceDB) for meaning matching
- Filter combinators:
  - Date range (created/modified)
  - Tag filter (single or multiple)
  - Folder filter
  - Has audio recording
  - Has attachments (photos, files)
  - Is pinned
- Save frequent filter combos as "Smart Folders" (dynamic, auto-updating)

### Graph View (Future — V2+)
- Visual network of all notes connected by backlinks and semantic similarity
- Interactive: tap node → open note, drag to rearrange
- Implementation: HTML Canvas or SVG rendering in WKWebView
- Not MVP — requires backlinks + embeddings to be useful
- Inspired by Obsidian's graph view

### Daily Notes Timeline
- Notes grouped by day in the list view
- Sticky date headers: "Today", "Yesterday", "8 May 2026"
- Natural for voice notes (inherently timestamped)
- Optional: calendar view showing which days have notes (inspired by Agenda)

## Note Display: AI Title + Date

### Current
```
┌─────────────────────────────┐
│ 8 mai 2026, 14:30           │  ← datetime as title
│ Début de la transcription...│
│ 8 mai · Travail             │
└─────────────────────────────┘
```

### Target
```
┌─────────────────────────────┐
│ Réunion budget Q3           │  ← AI-generated title (bold)
│ 8 mai 2026, 14:30           │  ← date below title (gray, small)
│ On a discuté des objecti... │  ← content preview
│ #meeting #budget · Travail  │  ← tags + folder
└─────────────────────────────┘
```

### Rules
- If AI title exists → show AI title (bold) + date below (gray xs)
- If AI title fails or hasn't processed → show datetime as title (current behavior)
- If note has no voice recording (pure text) → still show creation date below title
- Tags displayed as colored chips or inline text
- Folder shown as blue text (already implemented)

## Media in Notes

### Photos and Files
- Attach photos from library, take pictures with camera, import files
- Display: thumbnail grid below text content
- Storage: iOS Documents dir (same as audio files)
- Full-size view on tap
- See [04-media-attachments.md](04-media-attachments.md) for implementation details

### Audio Visualization
- Already implemented: 12-bar voice-reactive waveform (RMS analysis)
- Future: store waveform data for playback visualization
- Future: audio playback from note (play back the original recording)

## Export

### Markdown Export
- Each note → `.md` file with YAML frontmatter:
  ```yaml
  ---
  title: Réunion budget Q3
  date: 2026-05-08T14:30:00
  tags: [meeting, budget]
  folder: Travail
  ---
  Note content here...
  ```
- Bulk export: all notes as a zip of `.md` files
- Compatible with Obsidian vault import

### PDF Export
- Via CSS `@media print` + `window.print()` in WKWebView
- Or: Rust crate `printpdf` for custom layout
- Include title, date, content, embedded images

### Share
- Web Share API (`navigator.share()`) for quick sharing to Messages, Mail, etc.
- Share text, share with attachments

## Architecture Evolution

### Current Clean Separation
```
models/    → domain entities (pure Rust, no UI)
db/        → persistence layer (SQLite)
services/  → business logic (audio, transcription)
platform/  → iOS-specific (objc2)
ui/        → Dioxus components
```

### Planned Additions
```
services/
  ai.rs           → OpenAI client (titles, tags, cleanup, embeddings, chat)
  lancedb.rs       → Vector storage and retrieval
  export.rs        → Markdown/PDF export
db/
  attachment_repo.rs → Attachment CRUD
  tag_repo.rs        → Tag queries and smart folders
```

### SwiftUI Migration Path (if needed)
The `models/`, `db/`, `services/` layers are pure Rust with no Dioxus dependency. They can be compiled as a `staticlib` and called from Swift via C FFI. Only the `ui/` layer would need rewriting in SwiftUI.

## What NOT to Build

### Over-Engineering
- CRDT sync (Yjs, Automerge) — 2-3 months of engineering for a hypothetical need
- Real-time collaboration — not the product category
- Custom E2E encryption — Keychain + SQLite encryption suffice
- Plugin system — not needed at this scale

### Pretending to Be Native
- Simulated swipe-to-delete in JS — feels wrong, never matches UIKit physics
- Complex spring animations — CSS transitions are sufficient and honest
- Fake bottom sheets with momentum scrolling — keep it simple

### Wrong Product Category
- Always-on recording (Limitless) — privacy nightmare, battery drain
- Meeting bot (Otter) — requires calendar/conferencing integration
- Apple Watch app — needs SwiftUI, out of scope
- Desktop app — focus on mobile first (desktop via `dx serve --desktop` for dev only)

## Roadmap Summary

### Now (Quick Wins)
Pin notes, toast undo, daily timeline, empty states, date below title

### Next (1-2 weeks)
FTS5 search, inline tags, AI titles + cleanup, continue/append voice

### Track E (2-3 weeks)
OpenAI embeddings + LanceDB, semantic search

### Track F (3-4 weeks)
RAG chat, related notes, action items

### Polish (ongoing)
Photo attachments, markdown rendering, sort/filter, multi-select

### Future (if traction)
Backlinks, graph view, Smart Folders, export, SwiftUI port
