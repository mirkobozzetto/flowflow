# 04 — Document Import

How to import external documents (PDF, TXT, DOC) into FlowFlow for RAG indexing.

## Purpose

Imported documents become part of the knowledge base alongside voice/text notes. They get chunked, embedded, and are searchable via RAG chat. This is not about media display — it's about feeding the intelligence layer.

## Import Mechanism

### WKWebView File Input (Zero objc2)

```rust
input {
    r#type: "file",
    accept: ".pdf,.txt,.doc,.docx,.md,.csv,.json",
    onchange: move |evt| {
        // Read file data
        // Save to Documents dir
        // Extract text content
        // Store in documents table
        // Trigger embedding pipeline
    },
}
```

On iOS WKWebView, `<input type="file">` opens the native Files app picker. No objc2 needed.

### Supported Formats

| Format | Extension | Text Extraction | Rust Crate |
|--------|-----------|----------------|------------|
| Plain text | .txt, .md, .csv | Direct read | None (std::fs) |
| PDF | .pdf | Parse + extract | `pdf-extract` or `lopdf` |
| Word | .docx | XML parse | `docx-rs` |
| JSON | .json | Structured parse | `serde_json` (already used) |

Priority: TXT first (trivial), PDF second (most common), DOCX third.

## Data Model

### SQLite Schema (V2 migration)

```sql
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    content_text TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',
    size_bytes INTEGER NOT NULL DEFAULT 0,
    folder_id TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    modified_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_documents_folder ON documents(folder_id);
```

### Rust Model

```rust
pub struct Document {
    pub id: String,
    pub title: String,
    pub file_name: String,
    pub file_path: String,
    pub mime_type: String,
    pub content_text: String,
    pub tags: Vec<String>,
    pub size_bytes: i64,
    pub folder_id: Option<String>,
    pub created_at: String,
    pub modified_at: String,
}
```

## Storage

- Files saved to: `~/Documents/flowflow/imports/{uuid}.{ext}`
- Same pattern as audio files in `src/services/audio.rs:output_dir()`
- Extracted text stored in SQLite `content_text` field
- Original file kept for reference

## Import Pipeline

```
File selected via <input type="file">
  → Save raw file to Documents dir
  → Detect MIME type (extension-based)
  → Extract text content:
      TXT → read directly
      PDF → pdf-extract crate
      DOCX → docx-rs crate
  → Generate title from filename (or first line)
  → Store in documents table
  → Auto-generate tags via LLM (same call as notes)
  → Chunk text (512 tokens)
  → Embed chunks via OpenAI
  → Store vectors in LanceDB with metadata (doc_id, folder_id, tags)
  → Document now searchable via RAG
```

## Integration with Notes

### Attach to Note
A note can reference imported documents:
- Junction table `notes_documents (note_id, document_id)`
- Or simpler: document has optional `note_id` field
- In NoteDetail: "Import" button adds document linked to current note
- Linked documents visible in note view

### Standalone Import
- From sidebar "Documents" section
- Import without linking to a note
- Assign to folder, add tags
- Document exists as independent entity in RAG

## Cargo Dependencies

```toml
# PDF text extraction
pdf-extract = "0.7"

# DOCX parsing (if needed)
docx-rs = "0.4"
```

Start with TXT only (zero new deps), add PDF extraction when validated.

## RAG Integration

Imported documents are treated identically to notes in the RAG pipeline:
- Same chunking logic (512 tokens)
- Same embedding model (text-embedding-3-small)
- Same LanceDB storage
- Differentiated by `entity_type: "document"` in vector metadata
- Searchable alongside notes in chat (global, folder-scoped, tag-scoped)
