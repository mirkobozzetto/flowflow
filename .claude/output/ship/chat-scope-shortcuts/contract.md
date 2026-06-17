# Contract - chat-scope-shortcuts (inline ship)

artifact_kind: inline
engine_tier: solo
date: 2026-06-12

## Verified (no code change)

RAG scope: folder-scoped chat filters retrieval to that folder's notes
(rag::query folder_id -> list_notes_in_folder -> allowed_note_ids -> hybrid_search). Confirmed.

## Tasks

| ID | Task | Edit scope |
|----|------|-----------|
| T01 | Persist chat scope per conversation (KV settings key chat_scope:{conv_id}, no migration, no sync impact); restore on conversation open; clear on conversation delete; new chats start unscoped | src/db/conversation_repo.rs, src/ui/chat/view.rs |
| T02 | Settings page "Keyboard shortcuts" (macOS only) listing all shortcuts | src/ui/state.rs, src/ui/settings/{mod,shortcuts}.rs, locales |
| T03 | Double-tap Esc returns to the notes list | src/ui/mod.rs |

## Acceptance

1. Open chat A, set theme "pro", switch to chat B (other/no theme), come back to A -> chip "Theme : pro" is restored automatically and the RAG searches pro notes.
2. A brand-new chat starts unscoped; the scope chosen before/after the first message is remembered for that conversation.
3. Deleting a conversation removes its stored scope.
4. Settings (Mac) shows a Shortcuts section listing Cmd+N, Cmd+F, Cmd+,, Esc, Esc Esc, Enter, Shift+Enter; section absent on iPhone.
5. Pressing Esc twice quickly from any inner view returns to all notes.
6. make check green, cargo test green, both builds installed.

## Out of scope

- Syncing the per-conversation scope across devices (KV is device-local by design here)
- Any commit/push (git-guard)
