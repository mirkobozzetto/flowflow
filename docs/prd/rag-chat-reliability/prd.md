---
feature: RAG Chat Reliability
slug: rag-chat-reliability
type: prd
status: ready
stepsCompleted: [0, 1, 2, 3, 4]
sources: [github#40, github#41]
---

# PRD: RAG Chat Reliability

One product theme behind two reported bugs: a user must be able to trust the chat.
It has to find what they wrote, and answer in the language they asked in.

## Problem statement

Real downloaded-app users hit two trust-breaking failures in the RAG chat:

- A non-French user asked a question and the bot replied in French
  ("the chat bot started talking to me in French haha"). The hardcoded French
  scaffolding injected into the RAG user message biases the output language,
  worst when there are few or zero notes (the French labels are then the only
  non-question text). [#40]
- A short factual note ("I have a flight on 20 june to malaysia", 38 chars) is
  visible in the list but the chat answers "I couldn't find any notes" on a
  near-exact query. Notes whose trimmed content is under 50 characters are never
  embedded, so they never enter the vector store and are invisible to chat
  forever. [#41]

Why now: both come from genuine user reports on the shipped app. The second is
the more damaging - it silently breaks exactly the short factual notes (flights,
names, reminders) that RAG is supposed to be best at, while looking like the
feature works for normal-length notes.

## Goals

- The chat answers in the language of the user's question, for any supported
  language, regardless of the language of the stored notes or of internal
  instructions.
- Every note worth searching (short factual notes included) is findable by the
  chat via a near-exact query.
- Short notes created before the fix become findable without manual re-editing.
- No regression to existing RAG behavior, to multi-device sync (RFC 0004), or to
  data integrity (prime directive: zero data loss).

## Non-goals / Out-of-scope

- SQLite LIKE fallback over the notes table when vector search returns 0
  (decided out-of-scope for v1: do the real embedding fix and measure first).
- Explicit in-code language detection (prompt-only approach chosen; the model
  handles mirroring once the French bias is removed).
- Cleanup of the dead `RAG_SYSTEM_PROMPT` constant (unused; cosmetic).
- Any change to the embedding model, chunking strategy, or hybrid-search ranking.
- Embedding of audio files (already descoped elsewhere).

## User stories

1. As a non-French-speaking user, I want the chat to reply in the language I
   asked in, so that the app is usable in my language out of the box.
2. As a user with a short factual note, I want the chat to find it on a
   near-exact query, so that I can trust search instead of scrolling the list.
3. As an existing user, I want my already-created short notes to become findable
   after the fix, without re-opening or re-editing each one.
4. As any user, I want these fixes to leave normal notes, sync, and my data
   untouched, so that nothing I rely on breaks.

## Acceptance criteria

Story 1 (language):
- Given a fresh install with 0 notes, when I ask a question in English, then the
  answer is in English.
- Given any supported language (EN, FR, ES, DE) for the question, when I ask,
  then the answer is in that same language.
- Given French notes and a French question, when I ask, then the answer stays in
  French (no regression for the original-language case).

Story 2 (short-note retrieval, forward):
- Given a new note "flight to malaysia 20 june" (under 50 chars), when I ask
  "when is my flight?", then the chat returns that note.
- Given that note is saved, when embedding runs, then the device logs show no
  "embed skip: too short" line for it.
- Given a genuinely near-empty note (e.g. under ~3 words / ~10 chars after trim),
  when it is saved, then it is skipped (no junk vectors).

Story 3 (migration of existing short notes):
- Given short notes created before the fix (zero chunks today), when the app is
  launched once after the fix, then those notes are embedded and become findable
  without manual re-editing.
- Given the reproduced case ("I have a flight on 20 june to malaysia", 38 chars
  created earlier), when I ask "when is my next flight?" after one launch, then
  the chat returns it.

Story 4 (no regression):
- Given existing normal-length notes, when the fixes are deployed, then they
  remain findable exactly as before.
- Given a multi-device sync round-trip, when the re-embed pass runs, then no note,
  tag, or vector is lost or duplicated.

## Success metrics

- Question-language == answer-language: 100% match across a 4-language test panel
  (EN, FR, ES, DE), fresh-install / 0-note case included.
- 100% of notes whose trimmed content is at or above the floor are retrievable by
  a near-exact query.
- 0 "embed skip: too short" log lines for real factual notes on device.
- The reproduced flight note returns on "when is my next flight?" (was 0 results).
- 100% of pre-existing short notes become findable after exactly 1 post-fix launch
  (no per-note manual action).
- 0 notes / tags / vectors lost across the re-embed pass and a sync round-trip.

## Constraints & assumptions

- 100% Rust, Dioxus 0.7, targets iOS + desktop macOS; behavior must be correct on
  both.
- Local-first; embeddings always go through OpenAI (network + API cost). The
  migration pass is a bounded burst over zero-chunk notes only.
- Must not break RAG retrieval, RFC 0004 sync, or data integrity.
- Re-embed for existing notes is done by extending the existing reconcile pass to
  embed zero-chunk notes at next launch (chosen over lazy-on-open and
  forward-only).
- Recommended embedding floor: trim, then skip only if under ~3 words / ~10 chars.
  Note `String::len()` returns bytes, so a char/grapheme-aware check matters for
  non-ASCII.

## Open questions

- Exact floor value and unit (3 words vs ~10 chars; bytes vs chars for non-ASCII
  scripts). To lock in RFC/impl.
- Migration pass UX: silent background vs a visible "rebuilding index" state, and
  offline behavior (no network at launch -> skip and retry next launch, never
  block the UI).
- Embed `title + content` for all notes uniformly, or only for short ones?
  (Leaning uniform for simpler, higher recall.)
