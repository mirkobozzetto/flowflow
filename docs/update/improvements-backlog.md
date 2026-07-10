# Improvement backlog - 7 product ideas (2026-07-03)

Source: codebase exploration + market scan (Humm, Mindverse, Lightnote, Wisen,
DailyVox, Second Brain AI, Remi8). Dominant 2026 pattern: notes must COME BACK
to the user (push), not just be stored and queried (pull). FlowFlow is strong
on pull (RAG chat) and absent on push.

Priority pick: #1 now, #2 next, #4 as strategic background. 3/5/6/7 in reserve.

## 1. Related notes - the free graph (IN PROGRESS)

On NoteDetail, a "Related notes" section shows the 3 semantically closest
notes (LanceDB `hybrid_search` on the note's content, excluding itself).
Tap opens the note. Empty when nothing is close - never noise.

- Cost: minimal, the vector store and embeddings already exist.
- Market: auto-linking is the core pitch of Mindverse and Wisen.
- User story: dictate "idea: pitch a RAG audit to client X" -> below the note
  appear "client X budget meeting" (2 months old), "consulting pricing",
  "audit checklist". Forgotten context resurfaces with zero filing effort.

## 2. Daily digest - the morning note

Local notification at a fixed hour ("Your summary of yesterday"). Tap -> an
auto-generated digest: 2-3 sentences on yesterday's notes, recurring themes
of the week, today's reminders, optionally one old note that became relevant
again. No notes yesterday = no notification, no LLM call.

- Uses: existing LLM client, dated notes, EventKit reminders, local
  notifications.
- Market: Humm sells its Daily Digest as the Pro feature; Lightnote's whole
  pitch is "Daily Insights".

## 3. Frictionless capture - Share Extension + App Intents + widget

Today capturing requires opening the app. Add:
- iOS Share Extension: share text/URL/PDF from any app -> FlowFlow note
  (attachment pipeline already exists).
- App Intents: "Siri, note that..." -> dictation -> existing pipeline;
  Action Button support.
- Home-screen capture widget.

Thin Swift FFI, consistent with the "minimal native glue" rule. More capture
occasions -> bigger corpus -> better RAG. Market: Humm captures from Siri,
Action Button and Watch.

## 4. Local embeddings - the last cloud link (strategic)

Everything runs offline except embeddings (OpenAI). A quantized local
embedding model (bge-small / gte-small via candle, Metal) makes FlowFlow the
only "100% local second brain with cloud-free P2P sync". Competitors depend
on iCloud/Drive; nobody else has Noise P2P. Positioning, not just a feature.
Also kills the per-note embedding cost.

## 5. People cards - memory of people

Voice notes are full of people ("Jean"). Entity extraction at embed time ->
a "Jean" view aggregating every note mentioning him (things said, promises,
dates). Technical bonus: an entity index also fixes proper-noun retrieval,
a known RAG weak spot (#88). Market: Second Brain AI's "People Memory".

## 6. Tap-to-seek transcript

Tap a word in the transcript -> audio plays from that instant. Soniox and
Whisper both provide word timestamps; AVAudioPlayer already exists. Huge for
long voice notes (meetings). No competitor found does this well.

## 7. Resurfacing - the anti-graveyard

Minimal version: in the empty chat state or the digest, "this note from
6 months ago talks about the same thing". Technically: vector search of
recent notes against the old corpus. Market: Wisen's Echo rotation, Lightnote
"a note from a year ago resurfaces today".

## User stories (features 1 and 2, validated with Mirko 2026-07-03)

### Related notes
- When I record a voice note, the app computes semantic proximity with my
  existing notes, and I can see the 3 closest right under the note.
- When I open any note's detail, a "Related notes" section appears at the
  bottom, and tapping a card opens that note.
- When a note is truly isolated, the section stays empty - never noise.

### Daily digest
- When 7:30 comes and I have notes from yesterday, I get a notification
  "Your summary of yesterday", one tap opens the digest.
- The digest shows a short summary + recurring ideas this week, each point
  linking back to its source note.
- No notes yesterday -> no notification, ever.
- When an old note becomes relevant again (linked to yesterday's notes),
  the digest flags it.
