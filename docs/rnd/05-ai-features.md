# 05 — AI Features

LLM-powered features for FlowFlow. All use OpenAI API (already planned for embeddings). Async, non-blocking — notes remain usable while AI processes.

## Current State

- Transcription: Soniox REST API (stt-async-v4, French-optimized)
- Auto-title: datetime format ("8 mai 2026, 14:30") — functional but not intelligent
- Tags: stored as JSON array in SQLite, but never auto-generated
- Embeddings: planned (OpenAI text-embedding-3-small + LanceDB)

## Feature 1: AI Auto-Title

### Problem
Datetime titles are functional but uninformative. "8 mai 2026, 14:30" tells you when, not what.

### Solution
After transcription completes, call OpenAI to generate a short, meaningful title.

```
Prompt: "Generate a title for this note in 6 words or less. The note is in French.
Return ONLY the title, no quotes, no explanation.

Note content:
{content}"
```

### Display
- AI-generated title replaces the datetime title
- Creation date shown below the title in smaller gray text (always visible)
- If AI fails, keep the datetime title as fallback
- Visual indicator while processing: subtle spinner or "..." placeholder

### Implementation
- New service: `src/services/ai.rs` — OpenAI client (reuse reqwest)
- Call after Soniox transcription in `NoteDetail`'s `use_effect`
- Update note title via `db.update_note()`
- Environment variable: `OPENAI_API_KEY` in `.env`

## Feature 2: Transcript Cleanup

### Problem
Raw transcriptions contain hesitations ("euh", "hmm"), false starts, repetitions, and run-on sentences.

### Solution
LLM cleans up the transcript while preserving the speaker's voice and meaning.

### Rewrite Intensity Levels
Inspired by AudioPen:

| Level | What changes | What stays |
|-------|-------------|------------|
| **Low** | Remove "euh/hmm/ah", fix punctuation | Everything else verbatim |
| **Medium** | + Split run-on sentences, fix grammar | Key phrases, tone, personality |
| **High** | + Restructure into paragraphs, improve flow | Core meaning, facts, decisions |

```
Prompt (medium): "Clean up this French voice transcription.
Remove hesitations (euh, hmm, ah, ben), fix punctuation, split run-on sentences.
Keep the speaker's voice and tone. Do not add information.
Return ONLY the cleaned text.

Transcript:
{content}"
```

### UX
- Toggle in NoteDetail: "Clean up" button (or automatic after transcription)
- Store both raw transcript and cleaned version
- Allow switching between raw and clean
- Or: apply cleanup directly, keep raw in a hidden field for re-processing

## Feature 3: Auto-Tags Extraction

### Problem
Tags exist in the data model (`tags: Vec<String>`) but are never auto-generated.

### Solution
Extract 3-5 relevant tags from note content via LLM.

```
Prompt: "Extract 3-5 tags from this French note. Tags should be single words or short phrases.
Return as comma-separated list, no # prefix, no explanation.

Note:
{content}"
```

### Implementation
- Call in the same API request as auto-title (batch: title + tags in one prompt)
- Parse comma-separated response → `Vec<String>`
- Save via `db.update_note()` with `tags: Some(tags)`
- Display as colored chips in NoteCard and NoteDetail
- Tap a tag → filter notes by that tag

### Combined Prompt (Title + Tags + Cleanup)

Single API call for all three:
```
Prompt: "Process this French voice note. Return JSON only, no explanation.

{
  "title": "short title, 6 words max",
  "tags": ["tag1", "tag2", "tag3"],
  "cleaned": "cleaned transcript text"
}

Note:
{content}"
```

This reduces API calls from 3 to 1 and cuts latency.

## Feature 4: Action Items Extraction

### Problem
Voice notes from meetings contain action items buried in text.

### Solution
Extract action items as a structured checklist.

```
Prompt: "Extract action items from this French note as a bullet list.
Each item: who does what by when (if mentioned).
If no action items, return 'none'.

Note:
{content}"
```

### Storage
- Store as separate field or as a dedicated section at the top of the note
- Or: create linked "task" entities (future: tasks table)
- Display: checkbox list in NoteDetail

## Feature 5: Summary Templates

### Templates

| Template | Focus | Example Output |
|----------|-------|---------------|
| **Meeting** | Decisions, action items, attendees, next steps | Structured meeting minutes |
| **Journal** | Mood, reflections, key events | Personal journal entry |
| **Brainstorm** | Ideas grouped by theme, top 3 priorities | Organized idea map |
| **Interview** | Key quotes, impressions, follow-up questions | Interview debrief |

### Implementation
- Enum `SummaryTemplate` stored in SQLite (or just a string field)
- Picker in NoteDetail: dropdown or chips before/after recording
- Different LLM prompt per template
- Store summary separately from raw content (`summary TEXT` column)

## Feature 6: Continue / Append Voice

### Problem
Currently, recording always creates context for a new dictation. No way to add to an existing note's audio.

### Solution
When in NoteDetail of an existing note, the mic button says "Continue" instead of "Dictate". New transcription appends to existing content.

### Implementation
- Same mic button, different label based on `is_new`
- Append with separator: `\n\n` or timestamp marker
- Re-trigger AI processing (title + tags) after append if content changed significantly
- Audio files: save as separate WAV, link both to the same note via `update_audio_metadata`

## Feature 7: Semantic Search (Track E)

### Stack
- **Embeddings**: OpenAI text-embedding-3-small (1536 dimensions, ~$0.02/1M tokens)
- **Vector DB**: LanceDB (local, file-based, Rust-native)
- **Chunking**: 512 tokens per chunk for long notes

### Flow
```
Note saved/updated
  → Chunk content (if > 512 tokens)
  → Call OpenAI embeddings API
  → Store vectors in LanceDB with note_id + chunk_id
  → On search: embed query → top-k cosine similarity → return notes
```

### Hybrid Search
Combine FTS5 (keyword match) + LanceDB (semantic match) for best results. Rank by weighted score.

## Feature 8: Chat with Notes (Track F)

### Flow
```
User asks question
  → Embed question via OpenAI
  → Retrieve top-5 relevant chunks from LanceDB
  → Build context: system prompt + chunks + question
  → Call OpenAI chat completion
  → Display response with source citations
```

### UX
- Chat view: separate from note editor
- Accessible from sidebar or dedicated tab
- Shows sources: "Based on: Note X (8 May), Note Y (3 May)"
- Follow-up questions in same thread

## Feature 9: Related Notes (Proactive)

Inspired by Mem "Heads Up":
- When editing a note, sidebar shows 3-5 related notes (by vector similarity)
- Updates as content changes
- Tap to open related note
- Helps build connections between ideas

## API Cost Estimates

| Feature | API | Cost per note |
|---------|-----|--------------|
| Title + Tags + Cleanup | GPT-4o-mini | ~$0.001 |
| Embeddings | text-embedding-3-small | ~$0.0001 |
| Chat (Track F) | GPT-4o-mini | ~$0.005 per question |

At 100 notes/month: ~$0.10/month for AI features. Negligible.

## Environment Variables

```
OPENAI_API_KEY=sk-...
```

Add to `.env` (never committed). Add to `Dioxus.toml` plist if needed for iOS sandbox.

## Implementation Order

1. **AI service** (`src/services/ai.rs`): OpenAI client, title+tags+cleanup combined prompt
2. **Wire into NoteDetail**: call after transcription, update note
3. **Display**: AI title + date below + tag chips
4. **Continue/Append**: mic button label change, content append
5. **Track E**: LanceDB + embeddings on save
6. **Track F**: Chat view + RAG pipeline
