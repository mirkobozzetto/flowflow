---
artifact: "docs/prd/rag-chat-reliability"
artifact_kind: "prd"
locked: "2026-06-14"
---

# Definition of Done: RAG Chat Reliability

> Immutable target. Every item below is a concrete, checkable condition the final verification bundle validates against. Requirement changes get a NEW entry; never silently rewrite an existing line.

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | Fresh install, 0 notes, question in English -> answer in English | prd Story 1 | device manual (lang panel) |
| C2 | Question in EN/FR/ES/DE -> answer in that same language | prd Story 1 | device manual (lang panel) |
| C3 | French notes + French question -> answer stays French (no regression) | prd Story 1 | device manual |
| C4 | New note "flight to malaysia 20 june" (<50 chars) -> "when is my flight?" returns it | prd Story 2 | device manual + `make logs` |
| C5 | On save of that note, logs show NO "embed skip: too short" | prd Story 2 | `make logs` |
| C6 | Near-empty note (under ~10 chars trimmed) -> skipped, no junk vectors | prd Story 2 | device manual + `make logs` |
| C7 | Pre-existing zero-chunk short notes embedded after exactly 1 post-fix launch, no manual re-edit | prd Story 3 | device manual |
| C8 | Reproduced 38-char flight note returns on "when is my next flight?" after 1 launch | prd Story 3 | device manual |
| C9 | Existing normal-length notes still retrieve unchanged | prd Story 4 | device manual |
| C10 | Sync round-trip (iPhone <-> Mac): 0 note/tag/vector lost or duplicated after re-embed pass | prd Story 4 | device manual (2 devices) |
| C11 | `cargo test` green (rag_test labels + existing suite) | success metrics | `cargo test` |

## Out of scope (never build)

- SQLite LIKE fallback over the notes table when vector search returns 0.
- Explicit in-code language detection (prompt-only).
- Cleanup of the dead `RAG_SYSTEM_PROMPT` constant.
- Any change to embedding model, chunking strategy, or hybrid-search ranking.
- Embedding of audio files.

## Edit scope

- `src/services/rag.rs` - neutralize injected scaffolding labels (`build_context` + `query`).
- `src/services/constants.rs` - strengthen rule 3 of `RAG_AGENT_SYSTEM_PROMPT`.
- `tests/rag_test.rs` - update `build_context` label expectations.
- `src/services/embed.rs` - replace `< 50` gate with char-aware near-empty floor; embed `title + content`; factor shared embed-one-note core for reuse.
- `src/services/sync/reconcile.rs` - extend the boot pass to embed zero-chunk notes (offline-safe, non-blocking).
