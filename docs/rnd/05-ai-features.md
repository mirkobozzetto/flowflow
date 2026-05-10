# 05 — AI Features

LLM-powered intelligence layer for FlowFlow. All features use OpenAI API via reqwest (same pattern as Soniox client).

## Current State

- Transcription: Soniox REST API (stt-async-v4, French) — working
- Auto-title: datetime format ("8 mai 2026, 14:30") — functional, not intelligent
- Tags: stored as JSON array in SQLite, never auto-generated
- Embeddings: planned (OpenAI text-embedding-3-small + LanceDB)

## AI Service Architecture

Single service handling all OpenAI interactions:

```
src/services/ai.rs
  ├── OpenAiClient (reqwest-based, same pattern as SonioxClient)
  ├── process_note(content) → { title, tags, cleaned_text }
  ├── process_document(content) → { title, tags }
  ├── embed(text) → Vec<f32> (1536 dimensions)
  └── chat(question, context_chunks) → response
```

Environment variable: `OPENAI_API_KEY` in `.env`

## Feature 1: AI Post-Processing (Single Call)

After transcription completes OR document is imported, one API call handles everything:

```
POST https://api.openai.com/v1/chat/completions
Model: gpt-4o-mini
```

### Prompt (notes with voice)
```
Process this French voice note. Return JSON only.
{
  "title": "short title, 6 words max",
  "tags": ["tag1", "tag2", "tag3"],
  "cleaned": "transcript with hesitations removed, properly punctuated"
}

Content:
{content}
```

### Prompt (imported documents)
```
Process this document. Return JSON only.
{
  "title": "short descriptive title, 6 words max",
  "tags": ["tag1", "tag2", "tag3"]
}

Content (first 2000 chars):
{content_preview}
```

### Implementation
- Async, non-blocking — note/document remains usable during processing
- Fallback: keep datetime title if API fails
- Cost: ~$0.001 per note (GPT-4o-mini)
- At 100 notes/month: ~$0.10/month

## Feature 2: Embeddings (Track E)

### Pipeline
```
Content saved or imported
  → Split into chunks (512 tokens, overlap 50 tokens)
  → POST https://api.openai.com/v1/embeddings
      model: text-embedding-3-small
      → 1536-dimension vector per chunk
  → Store in LanceDB with metadata
  → Re-embed on significant content change (>20% diff)
```

### Chunking Strategy
```rust
fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    // Split by sentences first, then group into ~512 token chunks
    // Overlap of ~50 tokens between chunks for context continuity
}
```

### LanceDB Storage
Each vector record contains:
```rust
struct ChunkRecord {
    vector: Vec<f32>,        // 1536 dimensions
    entity_id: String,       // note or document ID
    entity_type: String,     // "note" or "document"
    folder_id: Option<String>,
    tags: Vec<String>,
    chunk_index: u32,
    text: String,            // raw chunk text for citations
}
```

### Tracking Table (SQLite)
```sql
CREATE TABLE IF NOT EXISTS embeddings_log (
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('note', 'document')),
    content_hash TEXT NOT NULL,
    chunk_count INTEGER NOT NULL,
    embedded_at TEXT NOT NULL,
    PRIMARY KEY (entity_id, entity_type)
);
```

Skip re-embedding if `content_hash` hasn't changed.

### Crate
```toml
lancedb = "0.x"  # check latest version compatible with iOS/aarch64
```

LanceDB is file-based (no server). Store in `~/Documents/flowflow/vectors/`.

## Feature 3: RAG Chat (Track F)

### Pipeline
```
User asks: "What did we discuss about the budget?"
  → Embed question via OpenAI (same model)
  → Query LanceDB: top-5 chunks by cosine similarity
      Filter by scope:
        Global → no filter
        Folder → WHERE folder_id = ?
        Tag → WHERE tags CONTAINS ?
  → Build prompt:
      System: "Answer based on the provided context. Cite sources."
      Context: [chunk1, chunk2, ..., chunk5]
      User: question
  → POST OpenAI chat completion
  → Parse response
  → Display with source citations
```

### Chat Prompt Template
```
You are a helpful assistant that answers questions based on the user's notes and documents.
Use ONLY the provided context to answer. If the answer is not in the context, say so.
Always cite which note or document the information comes from.
Answer in French.

Context:
---
[Note: "Réunion budget Q3" (8 mai 2026)]
{chunk text}
---
[Document: "Budget_Q3.pdf"]
{chunk text}
---

Question: {user_question}
```

### Chat Data Model
```sql
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('global', 'folder', 'tag')),
    scope_value TEXT,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    sources TEXT DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
```

Persist chat history per scope. User can review past conversations.

### Source Citations
Each AI response includes source references:
```rust
struct ChatSource {
    entity_id: String,
    entity_type: String,  // "note" or "document"
    title: String,
    chunk_preview: String, // first 100 chars of matched chunk
}
```

Displayed as tappable links below the AI response → navigates to source note/document.

## Feature 4: Document Content Extraction for RAG

Imported documents need text extraction before embedding:

| Format | Method | Crate |
|--------|--------|-------|
| TXT/MD/CSV | Direct read | std::fs |
| PDF | Text extraction | `pdf-extract` |
| DOCX | XML parsing | `docx-rs` |
| JSON | Structured → flat text | `serde_json` |

Extracted text stored in `documents.content_text` and fed into same embedding pipeline as notes.

## Feature 5: Tag-Based RAG Filtering

Tags stored as metadata in LanceDB vectors enable scoped search:

```rust
// Global search
let results = vector_store.search(query_vector, 5, None, None);

// Folder-scoped
let results = vector_store.search(query_vector, 5, Some(folder_id), None);

// Tag-scoped
let results = vector_store.search(query_vector, 5, None, Some("#meeting"));

// Combined: folder + tag
let results = vector_store.search(query_vector, 5, Some(folder_id), Some("#budget"));
```

This is the "smart context" system — tags and folders are not just organization tools, they're RAG filters.

## API Cost Estimates

| Feature | API | Cost per call |
|---------|-----|--------------|
| Title + Tags + Cleanup | GPT-4o-mini | ~$0.001 |
| Embedding (per chunk) | text-embedding-3-small | ~$0.00002 |
| Chat (per question) | GPT-4o-mini | ~$0.005 |

At 100 notes/month + 50 questions/month: ~$0.35/month total.

## Implementation Order

1. `src/services/ai.rs` — OpenAI client (title + tags + cleanup)
2. Wire into NoteDetail — call after transcription
3. `src/services/vectordb.rs` — LanceDB wrapper (store, search, delete)
4. Embed on note save + document import
5. `src/services/chat.rs` — RAG pipeline
6. `src/ui/chat.rs` — Chat view with scope selection
