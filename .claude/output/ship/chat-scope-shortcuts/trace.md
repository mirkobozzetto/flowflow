---
artifact: inline (chat scope memory + macOS shortcuts page, user prompt 2026-06-12)
kind: inline
engine_tier: solo
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: shipped (pending device validation by Mirko)
---

# Trace - chat-scope-shortcuts

| ID | Task | Status | Notes |
|----|------|--------|-------|
| T00 | Verify RAG scope | done | confirmed: folder scope filters retrieval (rag.rs:296-304 allowed_note_ids) |
| T01 | Scope per conversation | done | KV key chat_scope:{cid} (no migration, settings table, no sync triggers); restore on mount + on switch (validated against existing folders); persist on change + on create; cleared inside delete_conversation tx; new chats start unscoped; test chat_scope_test.rs |
| T02 | Shortcuts settings page | done | SettingsSection::Shortcuts, settings/shortcuts.rs (kbd chips), hub row gated cfg!(macos), 10 ftl keys EN/FR |
| T03 | Double-tap Esc -> notes list | done | JS lastEsc < 400ms -> escape-home -> navigate_with_slide(NotesList) |

## Verification

- make check: PASS (fmt + clippy 0 warnings)
- cargo test --features mobile: 269 passed (+1 chat_scope), 11 ignored
- make desktop-app + make all: see session log

## Checkpoints

- DB design choice: KV settings key instead of conversations column - avoids V12 migration and sync schema impact; tradeoff = scope does not sync across devices (device-local UI preference, like stt settings).
