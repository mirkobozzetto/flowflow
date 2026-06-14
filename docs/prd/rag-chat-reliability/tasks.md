---
feature: RAG Chat Reliability
slug: rag-chat-reliability
type: tasks
source_prd: docs/prd/rag-chat-reliability/prd.md
stepsCompleted: [0, 1, 2, 3]
sources: [github#40, github#41]
---

> Do NOT implement. This is the derived task list. Run `ship` (or the implementer) to execute.

## Relevant Files

- `src/services/constants.rs` - `RAG_AGENT_SYSTEM_PROMPT` rule 3 (language rule to strengthen)
- `src/services/rag.rs` - injected French scaffolding labels (`:354`, `:358`, `:380`, `:30`) + `query()`
- `tests/rag_test.rs` - `build_context` test asserting the old French labels
- `src/services/embed.rs` - the `content.len() < 50` gate (`:152` notes, `:242` attachments); embed title+content
- `src/services/sync/reconcile.rs` - `backfill_legacy_chunks` / `reconcile_once` to extend for zero-chunk notes
- `src/services/tools/search.rs` - retrieval path (verify only; change only if recall demands it)
- `src/services/embed.rs` callers - `ui/notes/detail.rs:230`, `ui/transcription_manager.rs:494`, `services/tools/create.rs:82`

## Tasks

- [ ] 1.0 Answer in the question's language _(PRD: Story 1)_
  - [ ] 1.1 Neutralize the injected structural labels in `rag.rs` to English internal markers (never user-facing): the empty-case label, the results label, and the question label
  - [ ] 1.2 Strengthen rule 3 in `RAG_AGENT_SYSTEM_PROMPT`: answer in the language of the QUESTION only; explicitly ignore the language of the notes and of these instructions; applies to any supported language
  - [ ] 1.3 Update `tests/rag_test.rs` `build_context` expectations to the new neutral labels
  - [ ] 1.4 Manual check: fresh install / 0 notes, ask EN -> EN; ES/DE -> same; FR question + FR notes -> FR

- [ ] 2.0 Embed short factual notes (forward fix) _(PRD: Story 2)_
  - [ ] 2.1 Replace the `content.len() < 50` gate at `embed.rs:152` (notes) with a near-empty floor: trim, then skip only under ~3 words / ~10 chars (char-aware, not byte-aware)
  - [ ] 2.2 Apply the same floor to the attachment gate at `embed.rs:242`
  - [ ] 2.3 Embed `title + "\n" + content` instead of `content` alone (metadata enrichment for recall on short notes)
  - [ ] 2.4 Confirm `purge_owner_chunks` no longer deletes chunks for notes that now pass the floor
  - [ ] 2.5 Manual check: new note "flight to malaysia 20 june" -> ask "when is my flight" -> found; `make logs` shows no "embed skip: too short" for it

- [ ] 3.0 Make existing short notes findable (migration pass) _(PRD: Story 3)_
  - [ ] 3.1 Extend the reconcile pass to actually embed notes with zero chunks at next launch (current backfill only moves existing vectors, it `continue`s on empty)
  - [ ] 3.2 Bound the pass to zero-chunk notes only; skip cleanly when offline and retry on the next launch; never block the UI
  - [ ] 3.3 Manual check: a pre-existing 38-char flight note becomes findable after exactly one post-fix launch, with no re-editing

- [ ] 4.0 Validate no regression + cross-device _(PRD: Story 4, Success metrics)_
  - [ ] 4.1 Verify normal-length notes still retrieve unchanged
  - [ ] 4.2 Run a sync round-trip (iPhone <-> Mac) and confirm zero loss/duplication of notes, tags, vectors after the re-embed pass
  - [ ] 4.3 Run the 4-language language-match panel (EN/FR/ES/DE) on device and desktop
  - [ ] 4.4 Confirm metrics: 100% language match, 0 "embed skip" for real notes, reproduced flight case returns
