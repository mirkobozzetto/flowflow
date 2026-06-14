---
artifact: inline (UX audit batch, user prompt 2026-06-12)
kind: inline
engine_tier: solo
stepsCompleted: [0, 1, 2, 3, 4, 5, 6]
final_status: shipped (pending device validation by Mirko)
---

# Trace - ux-polish-pass

| ID | Task | Status | Notes |
|----|------|--------|-------|
| T01 | Inline Transcribe error | done | transcribe_error signal, red line under button, cleared on retry |
| T02 | Localize STT/LLM errors | done | ui_lang helper, 9+1 ftl keys EN/FR, provider/client/manager/llm, test pins lang=fr |
| T03 | Desktop shortcuts | done | macOS eval keydown bridge: Cmd+N/Cmd+F/Cmd+,/Esc; #note-search id |
| T04 | Hover states | done | top_bar, sidebar (tabs/rows/footer), note_card, search, folder_picker, chat input, sources, copy btn |
| T05 | Sidebar New note + FAB lg | done | hidden lg:flex button (2-step nav from NoteDetail), FAB lg:hidden |
| T06 | Chat scope chip | done | chip Theme: name, opens picker, X clears scope, flex-col layout |
| T07 | Timestamps legibility | done | conversations, audio_section, sources, detail created-on, pairing, note_card -> 12px stone-500 |
| T08 | Chats search | done | chat_query filter on title + clear button + no-result state |
| T09 | Copy bot bubble | done | ui/clipboard.rs (navigator.clipboard + execCommand fallback), IconCopy, 1.5s Copied feedback |
| T10 | Chevron pivot | done | .chevron-pivot (rotate+transform 0.2s) on top_bar, folders, audio_section |
| T11 | Desktop scrollbar | done | thin 6px scrollbar lg+ (unlayered beats base hide), mobile keeps native |

## Verification

- make check: PASS (fmt + clippy 0 warnings)
- cargo test --features mobile: 268 passed, 11 ignored
- cargo check --features desktop: PASS
- make desktop-app: PASS, installed in /Applications/Flowflow.app
- make all (iPhone): PASS, installed on device (exit 0)

## Checkpoints

- GitNexus index unavailable (storage v41 vs runtime v40) - manual grep impact used, LOW risk (error strings displayed verbatim, one test updated).
- No new dependency, no DB change, no commit (git-guard).
