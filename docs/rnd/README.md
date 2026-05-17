# FlowFlow — R&D Notes

Research and development documentation for FlowFlow, a 100% Rust mobile app for voice + text notes with document import and RAG-powered chat.

## Documents

| Doc | Topic | Summary |
|-----|-------|---------|
| [01 — Market Landscape](01-market-landscape.md) | Competitive analysis | 24 apps analyzed (mainstream + AI/voice-first) |
| [02 — Feature Priorities](02-feature-priorities.md) | Core MVP features | Document import, tags, embeddings, RAG chat |
| [03 — Core UX](03-ux-patterns.md) | Essential UI patterns | Chat UI, import UI, tag management |
| [04 — Document Import](04-media-attachments.md) | File import system | Import PDF/TXT/DOC, text extraction, RAG integration |
| [05 — AI Features](05-ai-features.md) | LLM integration | Auto-titles, cleanup, tags, embeddings, RAG chat |
| [06 — Dioxus Platform](06-dioxus-platform.md) | Framework analysis | Limitations, objc2 patterns, roadmap 0.8 |
| [07 — Vision](07-vision.md) | Product direction | RAG architecture, chat scoping, tag interconnection, future API |
| [08 — Embedded Server & MCP](08-embedded-server-mcp.md) | Local API & MCP mobile | HTTP server on iOS, MCP protocol, Claude Desktop integration |
| [09 — Retrieval Quality](09-retrieval-quality.md) | Hybrid search & reranking | MemPalace analysis, BM25 crates, temporal boosting, RRF |
| [10 — Hybrid Search](10-hybrid-search.md) | Implementation plan | LanceDB native FTS+RRF, LLM reranking, adaptive sources, Legorag patterns |

## Status — What's Done vs Remaining

### Done (Tracks A–G + UI polish)

| Feature | Doc | Status |
|---------|-----|--------|
| Voice recording + transcription (Soniox) | 05 | Done |
| Text editing + auto-save | 03 | Done |
| Folder management (hierarchy, N:N) | 02 | Done |
| SQLite persistence (V1-V3 migrations) | 02 | Done |
| OpenAI embeddings + auto-embed on save | 05 | Done |
| LanceDB vector search (cosine) | 05 | Done |
| RAG chat + agent tools (search, create, summarize) | 05 | Done |
| Multi-provider LLM (OpenAI + Anthropic via rig) | 05 | Done |
| Auto-tags (LLM generated, manual add/remove) | 02 | Done |
| Document import (PDF, DOCX, TXT, MD, CSV) | 04 | Done |
| Auto-embed attachments | 04 | Done |
| Chat history (conversations + messages) | 03 | Done |
| In-app settings (API keys in SQLite) | 05 | Done |
| Filler word removal (FR + EN regex) | — | Done |
| Recording pause/resume/cancel + double-tap | — | Done |
| Shared recording component (DRY notes + chat) | — | Done |
| OKLCH warm palette + orange brand | — | Done |
| Directional page transitions | — | Done |
| iOS app icon (post-build injection) | — | Done |

### Not Started — Potential Next Steps

| Feature | Doc | Effort | Impact |
|---------|-----|--------|--------|
| Hybrid search (BM25 + cosine RRF) | 09 | Medium | High |
| LLM reranking (top 20 → LLM → top 3) | 09 | Low | High |
| Temporal boosting (recent notes ranked higher) | 09 | Low | Medium |
| Folder-scoped vector pre-filter | 09 | Low | Medium |
| Tag-scoped vector pre-filter | 02, 07 | Low | Medium |
| FTS5 full-text search | 02 | Medium | Medium |
| AI auto-title (LLM replaces datetime title) | 05 | Low | Medium |
| AI transcript cleanup (restructure, punctuate) | 05 | Low | Medium |
| Knowledge graph (entity-relationship) | 09 | High | Long-term |
| Embedded HTTP server (local API) | 08 | Medium | Medium |
| MCP server on iOS (Claude Desktop integration) | 08 | High | High |
| REST API / web interface | 07 | High | High |
| Dark mode | 07 | Medium | Medium |
| Cross-device sync | 07 | Very high | Long-term |

### Not Planned (Excluded from MVP)

Photo/camera, drag-drop, graph view, Apple Watch/Siri, CRDT sync, export.

## Date

2026-05-12
