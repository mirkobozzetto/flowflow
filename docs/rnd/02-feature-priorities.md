# 02 — Feature Priorities

Ranked features by impact/effort for FlowFlow. Bucket classification: A = pure Dioxus (HTML/CSS/Rust), B = needs objc2 FFI, C = needs Swift bridge or native extension.

## Impact / Effort Matrix

| # | Feature | Impact | Effort | Score | Bucket |
|---|---------|--------|--------|-------|--------|
| 1 | Pin notes | 5 | 1 | +4 | A |
| 2 | Full-text search (FTS5) with highlights | 5 | 2 | +3 | A |
| 3 | AI auto-title (replace datetime with LLM-generated title) | 5 | 2 | +3 | A |
| 4 | Toast undo after deletion (soft delete) | 4 | 1 | +3 | A |
| 5 | Transcript cleanup via LLM (remove hesitations) | 5 | 2 | +3 | A |
| 6 | Daily timeline grouping (notes grouped by day) | 4 | 1 | +3 | A |
| 7 | Inline tags `#tag` + auto-extraction | 4 | 2 | +2 | A |
| 8 | Continue/Append voice on existing note | 4 | 2 | +2 | A |
| 9 | Summary templates (meeting, journal, brainstorm) | 4 | 2 | +2 | A |
| 10 | Photo/file attachments | 4 | 2 | +2 | A/B |
| 11 | Date display below AI title | 3 | 1 | +2 | A |
| 12 | Search by criteria (date, tag, folder, has-audio) | 4 | 3 | +1 | A |
| 13 | Tags inside folders (filter by tag within folder) | 3 | 2 | +1 | A |
| 14 | RAG local + Chat with notes (Track F) | 5 | 4 | +1 | A |
| 15 | Long-press action sheet on note cards | 3 | 2 | +1 | A |
| 16 | Sort/filter via bottom sheet | 3 | 2 | +1 | A |
| 17 | Markdown rendering (basic: bold, italic, lists, headings) | 3 | 3 | 0 | A |
| 18 | Biometric lock per note (Face ID) | 3 | 2 | +1 | B |
| 19 | Share sheet (native iOS) | 3 | 2 | +1 | B |
| 20 | Camera capture in notes | 3 | 1 | +2 | A |

## Quick Wins (1-2 hours each)

### 1. Pin Notes
- Add `pinned BOOLEAN DEFAULT 0` + `pinned_at TEXT` to `notes` table (V2 migration)
- Sort: `ORDER BY pinned DESC, modified_at DESC`
- Pin icon on `NoteCard`, toggle in `NoteDetail` or long-press menu
- "Pinned" section header in `NotesList` when at least one note is pinned

### 2. Toast Undo After Deletion
- Replace hard `DELETE` with soft delete: add `deleted_at TEXT` column
- Show Toast component (slide-up, 5s timer, "Undo" button)
- Purge on app start: `DELETE FROM notes WHERE deleted_at < datetime('now', '-1 hour')`
- CSS animation: slide-up 150ms (consistent with existing transitions)

### 3. Daily Timeline Grouping
- Group notes by date in `NotesList` ("Today", "Yesterday", "8 May 2026")
- Sticky section headers via CSS `position: sticky`
- French date format (consistent with existing `generate_auto_title()`)

### 4. Date Below AI Title
- When note has an AI-generated title (not datetime), show the creation date below it
- Format: "8 mai 2026, 14:30" in gray-400 text-xs (already used in NoteCard)
- When title IS the datetime auto-title, don't duplicate — show nothing below

### 5. Empty States
- "No notes" state: SVG illustration + "Tap + to start" + CTA button
- "No search results" state: "No matches" + "Clear search" link
- "Empty folder" state: folder icon + "This folder is empty"

## Medium-Term (1-2 days each)

### 6. Full-Text Search (FTS5)
- Enable FTS5 virtual table: `CREATE VIRTUAL TABLE notes_fts USING fts5(title, content, content=notes, content_rowid=rowid)`
- Trigger to sync on INSERT/UPDATE/DELETE
- SearchBar component in TopBar with 200ms debounce
- Highlight matches via SQLite `snippet()` function
- Filter results: all notes, current folder, specific tag

### 7. Inline Tags `#tag`
- Parse `#[a-zA-ZÀ-ÿ0-9_-]+` regex from content on save
- Store in `tags TEXT DEFAULT '[]'` (already exists in schema)
- Auto-extract on save, display as colored chips in NoteCard
- Tag cloud view in sidebar drawer (below folders)
- Tap tag → filter notes by tag
- Tags inside folders: filter notes within a specific folder by tag

### 8. AI Auto-Title + Transcript Cleanup
- OpenAI API call after Soniox transcription completes
- Single prompt: "Generate a short title (max 6 words) and clean up the transcript (remove hesitations, false starts, restructure into paragraphs)"
- Async, non-blocking — note remains usable during processing
- Visual indicator: spinner or "AI" badge on card while processing
- Fallback: keep datetime title if API fails
- Rewrite intensity control: low (minimal cleanup) / medium / high (full restructure)

### 9. Continue/Append Voice
- Mic button in NoteDetail already exists
- When recording on an existing note, append transcription to content with separator
- Re-trigger auto-title + tags if note content changed significantly
- UX: same mic button, but now it says "Continue" instead of "Dictate" on existing notes

### 10. Search by Criteria
- Advanced search: date range, tag filter, folder filter, has-audio, has-attachment
- Bottom sheet with filter controls
- Combine with FTS5 for text search + metadata filters
- Save frequent searches as "Smart Folders" (dynamic, like Apple Notes)

## Deferred (Track E/F)

### 11. Embeddings + LanceDB (Track E)
- OpenAI text-embedding-3-small API call on note save/update
- Store vectors in LanceDB (local file-based vector DB)
- Chunk long notes (512 tokens per chunk)
- Re-embed on significant content changes

### 12. RAG + Chat (Track F)
- Semantic search: query → embed → top-k similar chunks from LanceDB
- Chat: user question → retrieve relevant chunks → build context → LLM response
- Display sources with citations
- "Related notes" sidebar via vector similarity

### 13. Summary Templates
- 4 presets: Meeting, Journal, Brainstorm, Interview
- Different LLM prompts per template (action items for meeting, mood for journal)
- Picker in NoteDetail before or after recording
- Store summary separately from raw content
