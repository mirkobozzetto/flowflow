---
artifact: "docs/prd/rag-chat-reliability"
artifact_kind: "prd"
engine_tier: "solo"
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: "code-complete-awaiting-device-validation"
updated: "2026-06-14"
---

# Trace Ledger: RAG Chat Reliability

> Single source of truth for progress. A fresh session reads ONLY this file to resume. One row per task.

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Notes |
|------|---------------|--------|---------------|--------|-------|
| 1.1 Neutralize rag.rs labels | C1,C2,C3 | done | `src/services/rag.rs` | solo | "--- User notes ---" / "(no relevant excerpts)" |
| 1.2 Strengthen rule 3 | C1,C2,C3 | done | `src/services/constants.rs` | solo | mirror question language, ignore notes/instructions |
| 1.3 Update rag_test labels | C11 | done | `tests/rag_test.rs` | solo | 3 asserts updated, tests green |
| 2.1 Replace `<50` gate (notes) | C4,C5,C6 | done | `src/services/embed.rs` | solo | `too_short_to_embed` < 10 chars |
| 2.2 Same floor (attachments) | C6 | done | `src/services/embed.rs` | solo | shared floor helper |
| 2.3 Embed title + content | C4 | done | `src/services/embed.rs` | solo | `embed_text` + `embed_note_core` |
| 2.4 Confirm purge guard | C4 | done | `src/services/embed.rs` | solo | purge only in too-short branch |
| 3.1 Reconcile embeds zero-chunk | C7,C8 | done | `src/services/embed.rs`, `src/services/sync/reconcile.rs` | solo | `embed_missing_notes`, reuses core |
| 3.2 Offline-safe, non-blocking | C7 | done | `src/services/embed.rs` | solo | no key/consent -> 0, retry next launch |
| host checks | C11 | done | - | solo | fmt+clippy clean, 269 tests pass |
| device validation | C1-C10 | pending | - | user | iPhone (`make all`) + Mac (`make desktop-app`); see verification-bundle.md |

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-03 | engine | solo (override 2-group default) | compile coupling + per-increment device validation |

## HALT events

- none
