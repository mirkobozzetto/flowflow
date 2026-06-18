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

## 2026-06 - Backup, export & restore (RFC 0001, issue #33)

One-tap export of all data as a single archive (scrubbed SQLite snapshot with vectors as BLOBs + WAV files + CRC manifest) via the iOS share sheet or macOS save dialog; API keys, Noise keys and pairings never leave the device. Import = read-only validation, then a crash-safe atomic swap at next cold launch (fault-injection tested at every state), vector index rebuilt offline. Sync protocol v3 reconverges without resurrections: restored flag + floor exemption + HLC guard + confirmed re-pairing. 249 tests. See [rfcs/0001-data-backup-export/](rfcs/0001-data-backup-export/).

## 2026-06 - Web search in chat (Exa + RRF, issues #43-#47)

Optional live web search fused with the local RAG. A Settings toggle runs an Exa web search in parallel (`tokio::join!`) with the LanceDB retrieval; Reciprocal Rank Fusion (Cormack et al., SIGIR 2009 - fuse on rank, not score) merges the two non-comparable score spaces (cosine distance vs Exa score) into one ranked list (K=60, local weight 1.2 / web 1.0). Web off keeps the existing path byte-for-byte; an Exa failure degrades to local-only and never breaks the answer; latency is max(web, local). Web sources render in their own section and open in the default browser (UIApplication.openURL on iOS, `open` on macOS), and notes keep their theme on open. Also fixed a UTF-8 char-boundary panic when truncating accented text in rerank and note previews. Shipped as #43 (spike + iOS cross-compile), #44 (Exa key in Settings), #45 (RRF fusion), #46 (toggle), #47 (web sources UI).

## 2026-06 - Save chat to notes (#52)

Keep a single bot answer or a whole conversation thread as a durable note, reusing the note pipeline (a shared `save_as_note` helper). The saved note is filed in the chat's own scope folder - unscoped chat -> unfiled, a themed chat -> that theme - so RAG isolation is correct-by-construction via the folder allow-list; re-saving the same content is idempotent (content dedup). Titles are AI-generated in the background (`generate_title`, the same path as note auto-titling), with a date title as a graceful fallback when no AI key is set. The note's web sources are preserved: a new nullable `notes.sources_json` column (SQLite V12, added to the sync catalog so it travels between devices) stores the web results; NoteDetail renders them with the same browser-opening cards as the chat, and a card badge marks web-sourced notes. Note content now renders as clean markdown with an edit/preview toggle, and long note titles wrap instead of forcing a horizontal scroll. Delete clears the saved state reactively.

## 2026-06 - Note to chat navigation (#42)

A round-trip path between a note and the chat. A round chat-bubble button sits next to the "Dictate" pill in the note's bottom bar (saved notes only); tapping it opens the chat already scoped to that note's theme - the folder scope is set at entry from `detail_folder_id` and carried into the RAG allow-list, so the answer stays inside the note's theme instead of falling back to all notes. The chat header shows the theme name, and because the chat was entered from a note (`previous_view` is a `NoteDetail`), the left control becomes a back arrow that returns to the exact origin note in a single transition (no intermediate flash of the notes list), on iPhone and Mac. Scope handling was hardened so every "new chat" entry (note button, global chat icon, sidebar new conversation, ⇧⌘Enter) sets its own folder scope and `scope_init` only restores the scope of existing conversations - fixing a wipe that previously dropped a note's theme to "all notes".

## 2026-06 - Note threads (RFC 0006, #54)

A thread is a first-class, titled, chronological stream of related notes you read top to bottom and append to in place. New `threads` table plus a nullable `notes.thread_id` (SQLite V13); a note belongs to at most one thread, ordered by `created_at`. A thread only counts as a thread at two or more members - a lone member renders as a plain note and a thread that drops below two collapses back (on leaving it and at boot), so entering one by mistake never strands you. The notes list mixes flat notes and stacked thread cards (a thread reuses the note card with its members' aggregated audio/web/reminder icons, just layered); the thread detail is a vertical timeline. Chat can be scoped to a thread, reusing #42's `allowed_note_ids` allow-list through a single `ChatScope` enum (Folder or Thread) that replaced the previous folder-only signal, with an explicit empty-scope answer instead of a silent zero-source reply. Threads register as one new sync kind and `thread_id` travels in the note payload (version vectors, tombstones, RFC 0004); thread delete re-stamps each member so a peer converges its members back to flat even though apply runs FK-off with triggers silenced. Backup counts threads; the V13 trigger installs on upgrade. Entry point is a stacked-cards button in the note bottom bar (start a thread or add to an existing one); the Mac notes list also switched from CSS multi-column to a row-major equal-height grid so notes read left to right like a comic strip. Adversarial 3-reviewer design pass on the RFC caught three sync blockers, all fixed before code. See [rfcs/0006-note-threads/](rfcs/0006-note-threads/).
