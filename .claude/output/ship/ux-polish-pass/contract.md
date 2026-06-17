# Contract - ux-polish-pass (inline ship)

artifact_kind: inline
engine_tier: solo
date: 2026-06-12

## Tasks

| ID | Task | Edit scope |
|----|------|-----------|
| T01 | Inline error on Transcribe button (red line, never silent) | src/ui/notes/audio_section.rs |
| T02 | Localize STT/LLM error strings EN+FR (no hardcoded French) | src/services/transcription/{provider,client}.rs, src/services/llm.rs, src/ui/transcription_manager.rs, src/services/i18n/mod.rs, locales/{en,fr}.ftl, tests/stt_provider_test.rs |
| T03 | Desktop shortcuts Cmd+N / Cmd+F / Cmd+, / Esc | src/ui/mod.rs, src/ui/note_list.rs |
| T04 | Hover states on interactive controls (desktop) | top_bar, sidebar/*, note_card, note_list, folder_picker, chat_input, chat/bot_bubble |
| T05 | Sidebar "New note" button (lg) + FAB hidden on lg | src/ui/sidebar/mod.rs, src/ui/fab.rs, locales |
| T06 | Visible clickable chat scope chip | src/ui/chat/view.rs, locales |
| T07 | Timestamps legibility (10px stone-400 -> 12px stone-500) | conversations, audio_section, sources_accordion, notes/detail, sync/pairing, note_card |
| T08 | Search input in chats list | src/ui/sidebar/conversations.rs, locales |
| T09 | Copy action on bot bubble (with feedback) | src/ui/chat/bot_bubble.rs, src/ui/clipboard.rs (new), src/ui/icons.rs, src/ui/mod.rs, locales |
| T10 | Smooth chevron pivot animation (top bar + folders + audio) | top_bar, sidebar/folders, audio_section |
| T11 | Thin visible scrollbar on every scrollable area (desktop) | tailwind.css |

## Acceptance

1. A failed transcription from the Transcribe button shows an inline red localized reason; the UI is never silently mute.
2. Every user-facing STT/LLM config error exists as an EN and FR ftl key; English UI never shows French.
3. On Mac: Cmd+N opens a new note, Cmd+F focuses notes search, Cmd+, opens settings, Esc closes pickers/menus/sidebar; interactive controls react on hover; sidebar has a New note button on lg and the FAB is hidden on lg.
4. A folder-scoped chat shows a visible chip (folder name) that opens the picker on click and has an X to clear scope.
5. Timestamps >= 12px stone-500; chats list searchable; bot bubbles have a copy button with visual feedback; chevrons pivot smoothly; desktop shows a thin scrollbar on all scroll areas.
6. cargo test green, clippy 0 warnings, desktop build + make all pass (run by ship per standing user instruction).

## Out of scope

- Per-conversation scope persistence (DB change - separate decision)
- Native macOS menu bar / DMG packaging (#36)
- Any commit/push (git-guard, explicit approval only)
