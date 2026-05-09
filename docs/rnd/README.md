# FlowFlow — R&D Notes

Research and development documentation for FlowFlow, a 100% Rust iOS app for voice + text notes with AI features.

## Documents

| Doc | Topic | Summary |
|-----|-------|---------|
| [01 — Market Landscape](01-market-landscape.md) | Competitive analysis | 24 apps analyzed (mainstream + AI/voice-first) |
| [02 — Feature Priorities](02-feature-priorities.md) | Impact/effort matrix | Ranked features, quick wins, medium-term roadmap |
| [03 — UX Patterns](03-ux-patterns.md) | Mobile UX | Patterns feasible in Dioxus WKWebView |
| [04 — Media Attachments](04-media-attachments.md) | Photos, camera, files | Import photos, take pictures, attach files to notes |
| [05 — AI Features](05-ai-features.md) | LLM integration | Auto-titles, transcript cleanup, tags, RAG, chat |
| [06 — Dioxus Platform](06-dioxus-platform.md) | Framework analysis | Limitations, JS in WKWebView, objc2, roadmap 0.8 |
| [07 — Vision](07-vision.md) | Product direction | Obsidian-inspired features, differentiator, long-term |

## Current Stack

- Language: Rust 1.94
- UI: Dioxus 0.7 (WKWebView on iOS)
- Styling: Tailwind CSS V4
- Audio: cpal 0.17 + hound 3.5
- Transcription: Soniox REST API (stt-async-v4, French)
- Database: SQLite (rusqlite 0.34, WAL mode)
- Embeddings (planned): OpenAI text-embedding-3-small
- Vectors (planned): LanceDB
- iOS platform: objc2 + objc2-avf-audio

## Date

2026-05-09
