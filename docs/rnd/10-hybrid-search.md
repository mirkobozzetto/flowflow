# 10 — Hybrid Search & LLM Reranking

Implementation plan for upgrading FlowFlow RAG from pure vector search to hybrid retrieval with adaptive source count.

## Current Pipeline

```
Question → OpenAI embed (1536d) → LanceDB cosine top 5 → Build context → Agent + tools → Response
```

| Parameter | Value | File |
|-----------|-------|------|
| Embedding model | text-embedding-3-small (1536d) | constants.rs |
| Chunk size | 375 words | constants.rs |
| Chunk overlap | 37 words (10%) | constants.rs |
| Top K | 5 (hard) | constants.rs |
| Distance metric | Cosine | vectordb.rs |
| Keyword search | None | — |
| Reranking | None | — |
| Source filtering | None (all 5 returned) | rag.rs |

### Known Weaknesses

- **Keyword miss**: "réunion Alice mardi" → embedding matches "meeting" semantically but misses exact "Alice"
- **Low diversity**: same note can appear multiple times via different chunks
- **No precision pass**: top 5 by raw distance = noisy, no relevance scoring
- **Fixed source count**: always 5, regardless of query complexity
- **No temporal signal**: old notes rank same as recent

## Target Pipeline

```
Question → OpenAI embed
         ├→ LanceDB vector search (top 15)
         └→ LanceDB FTS/BM25 search (top 15)
              ↓
         RRF fusion (merge + dedupe)
              ↓
         LLM reranking (top 15 → judge → top N)
              ↓
         Adaptive source count (3-15 based on relevance + user setting)
              ↓
         Agent + tools → Response with N sources
```

## LanceDB 0.27.2 Rust SDK — Native Hybrid Search

Already in Cargo.toml. No new dependencies needed.

### API (confirmed in source)

```rust
use lance_index::scalar::FullTextSearchQuery;
use lancedb::index::Index;
use lancedb::rerankers::RRFReranker;
use lancedb::query::{QueryBase, ExecutableQuery, QueryExecutionOptions};

// 1. Create FTS index (one-time, on table creation)
table.create_index(&["chunk_text"], Index::FTS(FtsIndexBuilder::default()))
    .execute().await?;

// 2. Hybrid search
let results = table.query()
    .full_text_search(FullTextSearchQuery::new(query_text.into()))
    .nearest_to(&query_vector)?
    .rerank(Arc::new(RRFReranker::default()))  // k=60
    .limit(top_k)
    .execute_hybrid(QueryExecutionOptions::default())
    .await?;
```

### Key Types

| Type | Import | Purpose |
|------|--------|---------|
| `FullTextSearchQuery` | `lance_index::scalar` | FTS query wrapper |
| `FtsIndexBuilder` | `lancedb::index` | FTS index config |
| `RRFReranker` | `lancedb::rerankers` | Reciprocal Rank Fusion (k=60 default) |
| `Reranker` trait | `lancedb::rerankers` | Custom reranker interface |
| `QueryExecutionOptions` | `lancedb::query` | Hybrid execution config |
| `execute_hybrid()` | `VectorQuery` method | Runs FTS + vector in parallel, merges via reranker |

### Cargo.toml Change

May need to enable features on lancedb:
```toml
lancedb = { version = "0.27.2", default-features = false, features = ["..."] }
```
Verify which features are needed for FTS + hybrid. Currently `default-features = false`.

## Step 1 — FTS Index + Hybrid Search

### vectordb.rs Changes

**New: create FTS index**
```rust
pub async fn ensure_fts_index(&self) -> Result<(), String> {
    let table = self.table().await?;
    table.create_index(&["chunk_text"], Index::FTS(FtsIndexBuilder::default()))
        .execute().await
        .map_err(|e| format!("FTS index error: {e}"))
}
```
Call after table creation in `store()` or on app startup.

**New: hybrid_search method**
```rust
pub async fn hybrid_search(
    &self,
    query_text: &str,
    query_vector: Vec<f32>,
    top_k: usize,
) -> Result<Vec<SearchResult>, String>
```

Internals:
1. Build `FullTextSearchQuery::new(query_text.into())`
2. `.nearest_to(&query_vector)` for vector part
3. `.rerank(Arc::new(RRFReranker::default()))` for RRF fusion
4. `.limit(top_k)` 
5. `.execute_hybrid(QueryExecutionOptions::default())`
6. Parse `RecordBatch` → `Vec<SearchResult>` (same struct as before)
7. Fallback to vector-only if FTS index missing or hybrid fails

### rag.rs Changes

```rust
// Before
let results = store.search(query_vec, RAG_TOP_K).await?;

// After
let results = store.hybrid_search(&question, query_vec, RAG_INITIAL_K).await?;
```

### constants.rs New Constants

```rust
pub const RAG_INITIAL_K: usize = 15;
pub const RAG_FINAL_K: usize = 8;
```

## Step 2 — LLM Reranking

After hybrid search returns 15 candidates, LLM scores relevance.

### rag.rs Addition

```rust
async fn llm_rerank(
    llm: &LlmClient,
    question: &str,
    results: &[SearchResult],
    final_k: usize,
) -> Result<Vec<SearchResult>, LlmError> {
    let passages = results.iter().enumerate()
        .map(|(i, r)| format!("[{}] {}: {}", i+1, r.title, &r.chunk_text[..200.min(r.chunk_text.len())]))
        .collect::<Vec<_>>().join("\n\n");

    let prompt = format!(
        "{}\n\nQuestion: {}\n\nPassages:\n{}\n\nReturn top {} indices, most relevant first. Format: 3,7,1,12,5,9,2,8",
        RERANK_PROMPT, question, passages, final_k
    );

    let response = llm.chat(&prompt).await?;
    let indices = parse_rerank_indices(&response, results.len());
    
    Ok(indices.into_iter()
        .filter_map(|i| results.get(i))
        .cloned()
        .collect())
}
```

### constants.rs

```rust
pub const RERANK_PROMPT: &str = "You are a relevance judge. Given a question and numbered passages, \
    rank them by relevance to the question. Return ONLY the passage numbers \
    separated by commas, most relevant first. No explanation.";
```

### Fallback

If LLM rerank fails (timeout, parse error), use RRF order and take first `RAG_FINAL_K`.
Cost: ~$0.001 per query (gpt-4o-mini on 15 short passages).

## Adaptive Source Count

### Logic

```rust
fn compute_source_count(results: &[SearchResult], user_max: usize) -> usize {
    let avg_distance = results.iter().map(|r| r.distance).sum::<f32>() / results.len() as f32;
    
    let adaptive_count = if avg_distance < 0.3 {
        5   // very relevant, fewer sources needed
    } else if avg_distance < 0.5 {
        8   // moderate, standard
    } else {
        12  // vague query, cast wider net
    };
    
    adaptive_count.min(user_max).min(results.len())
}
```

### Settings UI

- Slider in SettingsView: "Max sources" (3–15, default 8)
- Stored in SQLite: key `rag_max_sources`
- Read by rag.rs at query time

## Legorag Patterns Worth Adopting

| Pattern | Legorag Implementation | FlowFlow Adaptation |
|---------|----------------------|---------------------|
| Hybrid search | Qdrant dense + BM25 sparse | LanceDB native FTS + vector + RRF |
| RRF fusion | k=60, local+web weights | k=60 (LanceDB default) |
| Reranking | cross-encoder ms-marco | LLM reranking via existing LlmClient |
| Quality grading | 7-dimension scoring | Simplified: distance threshold + LLM judge |
| Query reformulation | LLM rewrite on low confidence | Future: if avg_distance > 0.6, rewrite + retry |
| Citation verification | SequenceMatcher 0.8 | Future: word overlap check |
| Grounding check | Per-sentence word overlap | Future: flag ungrounded claims |

## Industry Benchmarks (2026)

Source: OptyxStack, LLMversus, Clarity, MyEngineeringPath.

| Metric | Vector-only | + Hybrid | + Reranking | Full pipeline |
|--------|------------|----------|-------------|---------------|
| Recall@10 | 65-70% | 80-85% | 80-85% | 85-92% |
| NDCG@5 | 55-60% | 70-75% | 85-90% | 88-95% |
| Answer accuracy | 60-70% | 75-80% | 80-85% | 85-92% |

Production defaults (2026 consensus):
- BM25 top 50-100, vector top 50-100
- RRF fusion with k=60
- Cross-encoder or LLM rerank top 100-150 → top 8-12
- Chunk size 500-800 tokens, 10-15% overlap

## Implementation Order

| Step | Files | Effort | Impact |
|------|-------|--------|--------|
| 1 | vectordb.rs, rag.rs, constants.rs | 1 session | +15-20% recall |
| 2 | rag.rs, constants.rs | 0.5 session | +15% precision |
| 3 (future) | rag.rs | Trivial | Temporal relevance |
| 4 (future) | constants.rs, ai.rs, embed.rs | Trivial + re-embed | Better chunk quality |

## Risks

- **FTS index on iOS**: LanceDB FTS uses lance-core inverted index. Should work on iOS (no Tantivy dependency) but needs device validation.
- **LanceDB features**: `default-features = false` in Cargo.toml — may need to enable FTS-related features. Check compilation.
- **Re-indexing**: FTS index needs to be created on existing data. First hybrid_search call on old tables may fail until index exists.
- **LLM rerank latency**: Adds ~300-500ms per chat query. Acceptable for chat UX (user already waits 2-4s for LLM response).

## References

- [LanceDB Hybrid Search Docs](https://lancedb.com/docs/search/hybrid-search)
- [LanceDB RRF Reranker](https://lancedb.com/docs/integrations/reranking/rrf)
- [LanceDB Rust SDK PR #1940](https://github.com/lancedb/lancedb/pull/1940) — Hybrid search in Rust
- [Reranker trait](https://docs.rs/lancedb/latest/lancedb/rerankers/trait.Reranker.html)
- [trueno-rag](https://github.com/paiml/trueno-rag) — Pure Rust RAG pipeline
- [frankensearch](https://github.com/dicklesworthstone/frankensearch) — Two-tier Rust hybrid search
- [OptyxStack Hybrid Playbook](https://optyxstack.com/rag-reliability/hybrid-search-reranking-playbook)
- [docs/rnd/09-retrieval-quality.md](09-retrieval-quality.md) — Previous research (MemPalace, BM25 crates)
- Legorag source: `/Users/mirkobozzetto/code/Legorag`

## Status

| Feature | Status |
|---------|--------|
| FTS index creation | Not started |
| Hybrid search method | Not started |
| RRF fusion (native) | Not started |
| LLM reranking | Not started |
| Adaptive source count | Not started |
| Settings slider | Not started |
| Temporal boosting | Not started |
| Bigger chunks | Not started |
