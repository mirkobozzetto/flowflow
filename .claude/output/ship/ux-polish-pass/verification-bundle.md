# Verification bundle - ux-polish-pass

Automated checks already run by ship (standing instruction):
- make check (fmt + clippy): PASS
- cargo test --features mobile: 268 passed, 11 ignored
- cargo check --features desktop: PASS
- make desktop-app: installed in /Applications
- make all: iPhone install (see trace)

## Manual device validation (Mirko)

### Mac (/Applications/Flowflow.app)

1. Transcribe error inline: Settings -> set STT to Local (Whisper) -> delete/deselect model -> open a note with audio -> Transcrire -> red line with the localized reason appears under the button (no silent failure). Switch UI language to English -> error shows in English.
2. Shortcuts: Cmd+N (new note, also from an open note), Cmd+F (jumps to notes list + focuses search), Cmd+, (settings), Esc (closes folder picker / note menu / chat menu).
3. Hover: move the mouse over sidebar rows, tabs, note cards, top bar icons, folder picker rows, chat send/mic, copy button -> visible feedback everywhere.
4. Sidebar shows "Nouvelle note" button (orange, above Toutes mes notes); the floating + button is gone on desktop (still there on iPhone).
5. Chat scope chip: open a chat, pick a theme from the top bar title -> chip "Thème : X" appears above messages; click chip -> picker opens; click X -> scope cleared (chip disappears).
6. Chats list: search field filters conversations by title.
7. Bot bubble: Copier button copies the answer (paste somewhere), turns into green Copié for 1.5 s.
8. Scrollbar: thin grey scrollbar visible while scrolling notes list, chat, settings, sidebar.
9. Chevrons (top bar title, sidebar folders, note recordings) pivot smoothly on open/close.

### iPhone

1. No scrollbar regression (bars stay hidden), FAB still present.
2. Timestamps (chat list dates, audio dates, note created-on) readable.
3. Transcribe button error shows inline red line (e.g. airplane mode with Soniox provider).
4. Copy button works on a bot bubble (paste into a note).

No DB migration, no new dependency, no commit done.
