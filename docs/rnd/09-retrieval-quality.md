# 09 — Retrieval Quality & Hybrid Search

Research on improving RAG recall quality. Inspired by [MemPalace](https://github.com/MemPalace/mempalace) (52K stars, MIT, Python/ChromaDB) — the best-benchmarked open-source AI memory system.

## Current State (FlowFlow)

FlowFlow uses flat cosine similarity on OpenAI embeddings stored in LanceDB.

| Component | Implementation |
|-----------|---------------|
| Embedding | OpenAI text-embedding-3-small (1536 dims) |
| Storage | LanceDB 0.27.2 (local, on-device) |
| Search | Cosine similarity, top 5 chunks |
| Ranking | Raw distance score — no reranking |
| Keyword | None — pure semantic |
| Temporal | None — no date weighting |
| Structure | Flat corpus — all chunks in one table |

This works for basic Q&A but misses when the user asks for something with specific keywords that don't match semantically, or when they ask about recent content.

## MemPalace Architecture

MemPalace achieves 96.6% R@5 raw (no LLM) and 98.4% with hybrid heuristics on LongMemEval.

### Structured Index (Palace Model)

```
Palace
├── Wing (project/person)
│   ├── Room (topic)
│   │   └── Drawer (verbatim content)
│   └── Room
└── Wing
```

Searches can be scoped to a wing or room instead of running against the full corpus. FlowFlow's folder/tag system is analogous — we already have the metadata for scoping but don't use it aggressively enough.

### Hybrid Pipeline (Semantic + Keyword)

MemPalace combines three retrieval signals:

1. **Semantic similarity** (ChromaDB vector search) — same as what we do
2. **BM25 keyword matching** — exact term matching, handles proper nouns, technical terms
3. **Temporal proximity boosting** — recent content ranks higher

Merged via Reciprocal Rank Fusion (RRF):

```
RRF_score(d) = Σ 1 / (k + rank_i(d))
```

Where `k` is a constant (typically 60) and `rank_i(d)` is the rank of document `d` in retrieval method `i`.

### LLM Reranking

After hybrid retrieval returns top-20 candidates, an LLM reads each candidate and reranks by relevance to the query. Works with any capable model (Haiku, Sonnet, even local models via Ollama).

```
Query → Top 20 (hybrid) → LLM rerank → Top 3 (final) → Agent context
```

This pushes recall from 98.4% to ≥99%.

### Knowledge Graph

Temporal entity-relationship graph (SQLite-backed) with validity windows:

```
Entity: "Alice" → Role: "PM" → Valid: 2026-01-01 to present
Entity: "Budget Q3" → Amount: "50K" → Valid: 2026-07-01 to 2026-09-30
```

Enables: "Who was PM when we set the Q3 budget?"

## Rust Crates for BM25

| Crate | Version | Description | Fit for iOS |
|-------|---------|-------------|-------------|
| [bm25](https://crates.io/crates/bm25) | 2.3.2 | Scorer + search engine, simple API | Yes |
| [bm25_turbo](https://crates.io/crates/bm25_turbo) | 0.2.0 | 28K QPS on 8.8M docs | Yes |
| [bm25x](https://crates.io/crates/bm25x) | 0.3.1 | Streaming, mmap support | Yes |
| [ir-search](https://crates.io/crates/ir-search) | 0.15.0 | Full hybrid BM25 + vector + LLM rerank | Heavy deps |

The `bm25` crate is the lightest option. Pure Rust, no heavy dependencies, should compile for iOS.

## What We Could Add (Ranked by Impact/Effort)

### 1. Hybrid Search (BM25 + Cosine) — HIGH impact, MEDIUM effort

Add BM25 index alongside LanceDB. On each query:

```
Query → BM25 top 20 + LanceDB top 20 → RRF merge → Top 5
```

Implementation:
- Add `bm25` crate to Cargo.toml
- Build BM25 index from note/attachment content (rebuild on save)
- In `rag.rs`: query both, merge via RRF, pass top 5 to agent

Benefits: catches exact keywords missed by semantic search ("Q3", "Alice", technical terms).

### 2. LLM Reranking — HIGH impact, LOW effort

We already have the LLM client. After retrieving top 10 chunks:

```rust
let rerank_prompt = format!(
    "Given this question: {question}\n\n\
     Rank these passages by relevance (most relevant first). \
     Return only the indices: [best, second, third]\n\n\
     {passages_formatted}"
);
```

Use the existing `LlmClient::chat()` to rerank. ~1 extra API call per question.

### 3. Temporal Boosting — MEDIUM impact, LOW effort

Multiply vector similarity by a time decay factor:

```rust
let days_ago = (now - chunk.created_at).num_days() as f32;
let time_boost = 1.0 + (1.0 / (1.0 + days_ago / 30.0));
let final_score = similarity * time_boost;
```

Recent notes get ~2x boost, 30-day-old notes ~1.5x, old notes ~1x.

### 4. Scoped Search — MEDIUM impact, LOW effort

Already have folder_id and tags in vector metadata. Use them as pre-filters:
- If user is in a folder → filter by folder_id first
- If query mentions a tag → filter by tag
- Reduces noise, improves precision

Partially implemented (folder context passed to agent) but not used as vector pre-filter.

### 5. Knowledge Graph — LOW impact for now, HIGH effort

Temporal entity-relationship graph. Useful for "who/what/when" queries across many notes. Not worth building until the note volume justifies it (100+ notes with overlapping entities).

## Implementation Plan

### Phase 1: Quick Wins (1 session)
1. Temporal boosting in `vectordb.rs` search results
2. Folder pre-filter in vector search when user is in a folder
3. Increase top-k from 5 to 10, let agent pick the best

### Phase 2: Hybrid Search (1-2 sessions)
1. Add `bm25` crate
2. Create `services/keyword_index.rs` — BM25 index management
3. Update `rag.rs` — merge BM25 + vector results via RRF
4. Rebuild index on note save/delete

### Phase 3: LLM Reranking (1 session)
1. After hybrid retrieval, format top 10 as numbered passages
2. Ask LLM to rank by relevance
3. Take top 3 for final agent context
4. Measure recall improvement

## References

- [MemPalace](https://github.com/MemPalace/mempalace) — 52K stars, 96.6-99% R@5
- [MemPalace Rust rewrite](https://github.com/bunkerlab-net/mempalace) — 53 stars, early stage
- [bm25 crate](https://crates.io/crates/bm25) — Rust BM25 scorer
- [ir-search crate](https://crates.io/crates/ir-search) — hybrid BM25+vector+rerank
- [LongMemEval benchmark](https://github.com/xiaowu0162/LongMemEval) — memory retrieval evaluation
- [Reciprocal Rank Fusion](https://plg.uwaterloo.ca/~gvcormac/cormacksigir09-rrf.pdf) — merging multiple rankings

## Status

| Technique | Status |
|-----------|--------|
| Flat vector search (cosine) | Done |
| Auto-embed on save | Done |
| Agent with tools | Done |
| Temporal boosting | Not started |
| Folder pre-filter | Partial (context only, not vector filter) |
| BM25 hybrid | Not started |
| LLM reranking | Not started |
| Knowledge graph | Not started |
