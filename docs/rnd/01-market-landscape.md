# 01 — Market Landscape

Competitive analysis of 24 note-taking apps across mainstream, AI/voice-first, and privacy-first categories.

## Mainstream Apps

### Notion
- **Model**: everything is a draggable/transformable "block" (text, DB, page, image)
- **Killer features**: slash commands (`/`), inline databases, pages-in-pages, template gallery, AI Meeting Notes
- **Mobile UX**: no hover → always-visible `•••` and `+` icons, single column on mobile
- **Weakness**: no offline mode (2026)
- **Relevance to FlowFlow**: slash commands are now industry standard; block model is overkill for voice notes

### Obsidian
- **Model**: local-first vault of plain `.md` files, zero lock-in
- **Killer features**: wikilinks `[[Note]]` with bidirectional backlinks, graph view, Canvas (infinite 2D space), plugin ecosystem (thousands), source/live preview switching
- **Mobile UX**: tabs, command palette, customizable toolbar, plugins work on mobile
- **Roadmap**: mobile widgets, lock screen quick actions, startup time improvements
- **Relevance to FlowFlow**: backlinks, graph view, and local-first philosophy are direct inspiration. File format openness is a differentiator.

### Bear
- **Model**: Markdown with inline rendering, design-driven, Apple-only
- **Killer features**: inline `#tags` + nested tags (`#work/clients`), 250+ TagCons icons, hide markdown formatting, backlinks, inline sketching, OCR search in images/PDFs, 28+ themes
- **Mobile UX**: custom keyboard toolbar (BIU button), document scanner, lock screen widgets (Random note, New note, Search), Apple Watch app
- **Relevance to FlowFlow**: inline tags are the ideal model for FlowFlow. Custom keyboard toolbar is smart. OCR in images is aspirational.

### Apple Notes (iOS 18+)
- **Killer features**: live audio transcription (direct FlowFlow competitor), text highlights, Smart Script (Apple Pencil), auto-calculations, collapsible headings, Quick Note from Lock Screen, image search OCR, Face ID lock, Smart Folders (dynamic filters), document scan
- **Mobile UX**: pin notes, Share Sheet integration, deep iOS integration
- **Relevance to FlowFlow**: live transcription in Apple Notes raises the baseline. FlowFlow must differentiate with AI transformation (cleanup, titles, tags) not just raw transcription.

### Google Keep
- **Model**: colored cards, capture-first
- **Killer features**: 5 note types (Audio, Image, Drawing, List, Text), voice notes with auto-transcription, color coding (8+ colors), geo-located reminders, collaboration, Wear OS tiles
- **Mobile UX**: single-tap create (configurable), multi-select via touch-and-hold, sort by date/custom
- **Relevance to FlowFlow**: multi-type capture (audio/image/list/text) in one FAB is worth considering. Color coding aids memory.

### Simplenote
- **Model**: extreme minimalism, plain text only, 100% free
- **Killer features**: optional per-note markdown, automatic versioning (revert to any state), publish-to-web (1 link), cross-platform (iOS, Android, Mac, Win, Linux, web)
- **Mobile UX**: two-pane layout, no formatting toolbar → distraction-free
- **Relevance to FlowFlow**: proof that doing LESS can win. "Fast, free, no friction" is a valid strategy.

### Standard Notes
- **Model**: privacy-first, E2E encrypted (XChaCha20-Poly1305 + Argon2), open source audited
- **Killer features**: pluggable note types (Plain, Rich, Markdown, Spreadsheet, Code, Tasks, 2FA), Smart Views (custom queries), nightly encrypted backup, per-note password lock
- **Relevance to FlowFlow**: "note = container, editor is plug-in" architecture is interesting. Privacy-first positioning aligns with local RAG vision.

### Craft
- **Model**: block-based like Notion but design-driven, Apple-native, focus on writing
- **Killer features**: Daily Notes (calendar timeline), block drag-drop, backlinks, native tasks (Inbox, due dates), AI Assistant (summarize, rewrite, search), 1-click web publishing, whiteboards, focus mode
- **Mobile UX**: full feature parity offline, sync auto
- **Relevance to FlowFlow**: Daily Notes = pattern to adopt for voice notes (they already have timestamps). AI Assistant for search across knowledge base is Track F.

### Agenda
- **Model**: notes-meets-calendar, timeline-driven
- **Killer features**: "On the Agenda" shortlist, notes attached to dates/calendar events, project timeline, categories > projects > notes (3 levels), Reminders integration, Apple Pencil annotation
- **Relevance to FlowFlow**: "every note is an event" — powerful mental inversion. Voice notes inherently have a timestamp → timeline view is free.

## AI / Voice-First Apps

### AudioPen
- **Core value**: "fuzzy thought to clear text" — rewrites messy voice into clean paragraphs
- **Killer feature**: adjustable rewrite intensity (low/medium/high) — key: don't over-rewrite, preserve user's voice. Custom writing styles ("Write Like Me")
- **Pitfall**: too-neutral rewriting flattens personality. Solution: intensity slider + preserve key phrases.
- **Relevance to FlowFlow**: rewrite intensity control is essential. "Write Like Me" few-shot prompting is a premium feature.

### Cleft
- **Core value**: built for "verbal thinkers" / neurodivergents
- **Killer features**: on-device Whisper transcription, AI summary generates title + markdown structure, live transcription during recording, Continue/Append on existing note, Merge two notes, Rewrite (regenerate summary from transcript)
- **Relevance to FlowFlow**: Continue/Append is critical — resume a note without creating a new one. Edit raw transcript + regenerate is powerful for fixing proper nouns.

### Otter.ai
- **Model**: meeting transcription automation
- **Killer features**: OtterPilot auto-joins Zoom/Meet/Teams, real-time transcription + summary + action items
- **Limitation**: 30 min on free tier, only 3 languages
- **Relevance to FlowFlow**: meeting-first is not FlowFlow's lane. But action item extraction is universally useful.

### Reflect
- **Model**: E2E encrypted PKM with AI
- **Killer features**: AI palette (cmd+J highlight → transform), AI-generated backlinks, chat with notes, Whisper voice transcriber
- **Relevance to FlowFlow**: AI palette (select text → transform) is an elegant UX pattern.

### Mem
- **Model**: "organize nothing" — AI handles organization
- **Killer features**: Smart Tags (auto-generated), Mem Chat (semantic search), Heads Up (proactively surfaces related notes)
- **Philosophy**: "Capture is your job, organize is ours"
- **Relevance to FlowFlow**: proactive related notes via vector similarity is a Track F feature. "Organize nothing" aligns with FlowFlow's optional folders.

### Reor (open-source, AGPL)
- **Stack**: LanceDB + Ollama + Transformers.js — chunking + embedding + RAG Q&A
- **Relevance to FlowFlow**: identical architecture to what FlowFlow plans. Reor is desktop Electron though — FlowFlow would be the mobile equivalent.

### Atomic
- **Model**: iOS app, "every atom embedded into vector space"
- **Features**: semantic search, agentic chat with citations
- **Relevance to FlowFlow**: closest mobile competitor for the RAG vision.

### Granola
- **Model**: no bot — captures system audio directly, user takes short notes, AI enhances after
- **Killer features**: templates by meeting type (sales, 1:1, interview), "Chat with meeting" (Cmd+J), cited sources
- **Pitfall**: no export, limited integrations
- **Relevance to FlowFlow**: templates by context is worth adopting. Always provide export from day one.

### Plaud Note
- **Model**: credit-card-size hardware recorder
- **Killer features**: mind maps from transcription, 10,000+ summary templates, "multidimensional summaries" (same call → 3 role-specific outputs)
- **Relevance to FlowFlow**: mind map generation (Mermaid output from LLM) is aspirational. Multi-perspective summaries are interesting.

### Limitless
- **Model**: always-on wearable pendant
- **Killer features**: "Consent Mode" detects new voices, retrieval LLM ("what price did Jennifer mention?")
- **Pitfall**: weak speaker recognition, 30% iPhone battery drain
- **Relevance to FlowFlow**: retrieval from specific notes is Track F. Always-on is a different product category.

## Cross-Cutting Patterns

### Organization
- Tags > folders on mobile (Bear, Simplenote, Notion) — faster than navigating
- Hybrid tags + folders = best of both (Apple Notes Smart Folders)
- Pin notes = universal pattern
- Backlinks = post-Obsidian standard

### Quick Capture
- Lock screen widgets (Bear, Apple Notes, Obsidian roadmap)
- Voice memo + auto-transcription (Keep, Apple Notes iOS 18)
- Single-tap create (Keep)
- Multi-type capture FAB (Keep: text/audio/photo/drawing/list)

### AI Intelligence
- Live transcription is now baseline (Apple Notes iOS 18)
- Search by image content (Apple Notes OCR, Bear)
- AI Assistant for search/summarize/rewrite (Craft, Notion)
- Smart Folders / Smart Views = dynamic filters (Apple Notes, Standard Notes)

### Privacy
- E2E encryption + public audit (Standard Notes, Obsidian sync)
- Local-first + open file format (Obsidian, Simplenote)
- Per-note lock + biometrics (Apple Notes, Standard Notes, Bear)
