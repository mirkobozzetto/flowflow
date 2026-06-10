# FlowFlow history

> Everything that shipped, oldest first. One block per milestone, updated with each significant merged PR.

## 2026-05 - Foundations (Tracks A-D)

Dioxus iOS scaffold, mic capture (cpal + hound, WAV), Soniox REST transcription, SQLite storage, Tailwind V4 UI. The 5-layer architecture (models / db / services / platform / ui) lands here.

## 2026-05 - RAG and chat (Track E)

OpenAI embeddings + LanceDB on device, auto-embed on save, in-app Settings for API keys, tag chips with LLM auto-gen, chat UI with RAG pipeline and source citations, persistent conversations with sidebar tabs.

## 2026-05 - Agent and providers (Track F)

Migration to rig-core: agent tools (search_notes, create_note, summarize_folder), unified reqwest, Anthropic as second chat provider.

## 2026-05 - Document attachments (Track G)

SQLite V3 migration, attachment cards + modal viewer, native iOS file picker, PDF (pdf-extract, then PDFKit with OCR) and DOCX (zip + quick-xml) parsing, auto-embed of imported documents.

## 2026-05 - Architecture and audio rewrite (PR #3, #4)

SRP refactoring (6 files -> 22 then 64 modules), audio player (AVAudioPlayer FFI, V4 migration, WAV lifecycle), recording UX fixes, orange rebrand (#E85D0A) and gear icon, EUPL 1.2.

## 2026-05 - Multi-audio and i18n

V5/V6 migrations, multiple audios per note, chat scope per folder, smart mic routing, FR/EN i18n (fluent, 96+ keys, system-language detection).

## 2026-06 - App Store v1.0

Release pipeline (make appstore: distribution signing, IPA, validator green), CFBundleVersion auto-bump, screenshots, AI consent screen, review API keys flow. v1.0 approved and live (non-EU; EU pending DSA trader verification). See [guides/appstore.md](guides/appstore.md).

## 2026-06 - Smart reminders (RFC 0003)

LLM detection of reminder intents in notes ("pick up the kids at 5pm"), EventKit integration, one-tap confirm, recurrence basics. Device-validated.

## 2026-06 - Multidevice sync (RFC 0004)

Encrypted LAN P2P sync iPhone <-> Mac: Noise XXpsk3 over TCP, pairing by code/QR, version-vector merge, tombstones + GC, full-state reconcile, conflict policy, sync triggers, at-rest protection. SQLite V10. 200+ tests. See [rfcs/0004-multidevice-sync/](rfcs/0004-multidevice-sync/).

## 2026-06 - Desktop app hardening (PR #20)

Mac build without the iOS widget, durable data dir in Application Support.

## 2026-06 - Sync realtime UX (PRD sync-realtime-ux, issues #22 #23 #24)

Real-time UI refresh after inbound sync (< 1 s, global data-version signal), edit-collision banner with zero keystroke loss, full safe-area support (viewport-fit=cover, top + bottom insets), QR pairing via the flowflow:// URL scheme (camera scan -> prefilled one-tap confirm, cold-start included). Device-validated. See [prd/sync-realtime-ux/](prd/sync-realtime-ux/).
