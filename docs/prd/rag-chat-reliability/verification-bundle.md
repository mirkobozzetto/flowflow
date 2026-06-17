---
artifact: "docs/prd/rag-chat-reliability"
stack: "rust / cargo"
generated: "2026-06-14"
ran_by: "user"
---

# Verification Bundle: RAG Chat Reliability

> Host checks already run by ship (results below). The DEVICE checks are yours - they need a build on the iPhone/Mac and manual RAG queries. No commit/push until you have run these and approved.

## Host checks (already run by ship - all green)

| Command | Validates | Result |
|---------|-----------|--------|
| `cargo fmt --check` | formatting | pass |
| `cargo clippy --features mobile` | static/lint | pass, no warnings |
| `cargo test` | C11 + regression | 269 passed, 11 ignored |

## Build iPhone + Mac (you run)

Both targets ship the same Rust code, so C1-C9 must hold on BOTH. C10 (sync) needs the two running together.

iPhone:

| Command | Validates | Expected |
|---------|-----------|----------|
| `make ddev` | hot-reload build on iPhone (quick lang/retrieval checks) | app launches |
| `make all` | full signed install (for the migration boot-pass test) | app installs + launches |
| `make logs` | open Console.app filtered FlowFlow | log stream visible |

Mac:

| Command | Validates | Expected |
|---------|-----------|----------|
| `make desktop` | dev window, hot reload, real mic (quick lang/retrieval checks) | window opens |
| `make desktop-app` | standalone release .app in /Applications, data in `~/Library/Application Support/FlowFlow` (real boot reconcile + pairing/sync) | ">> Flowflow.app installed" |

Mac logs: run the standalone app from a terminal, or `Console.app` filtered "FlowFlow", to see the same `embed missing: ...` / `[reconcile] ...` lines.

## Manual checks (USER ONLY) - run on iPhone AND Mac

| # | Contract | Platform | Steps | Pass signal |
|---|----------|----------|-------|-------------|
| C1 | fresh-install language | both | 0-note state, ask in English | answer in English (not French) |
| C2 | language panel | both | ask the same question in EN, FR, ES, DE | each answer is in the asked language |
| C3 | no FR regression | both | French notes + French question | answer stays French |
| C4 | short note forward | both | new note "flight to malaysia 20 june", ask "when is my flight?" | note is returned |
| C5 | no skip log | both | watch logs while saving C4's note | NO "embed skip: too short" for it; "embed done ... (N chunks)" appears |
| C6 | near-empty floor | both | save a note "ok" (< 10 chars), watch logs | "embed skip: too short" appears; no chunks created |
| C7 | migration of old short notes | both | with a pre-fix zero-chunk short note present, launch once (`make all` / `make desktop-app`), wait for `embed missing: N notes embedded` | the note becomes findable, no re-edit |
| C8 | reproduced flight case | both | the 38-char "I have a flight on 20 june to malaysia" note, ask "when is my next flight?" after one launch | chat returns it (was 0 results) |
| C9 | no retrieval regression | both | ask about a normal-length note that already worked | still returned, unchanged |
| C10 | sync round-trip | iPhone + Mac | run the re-embed pass on both, then sync iPhone <-> Mac (`make desktop-app` paired) | 0 note/tag/vector lost or duplicated |

## Contract coverage

- C11 -> `cargo test` (covered, green).
- C1-C10 -> manual on iPhone AND Mac (above). Uncovered by automation by design (no on-device test harness).

## Decisions locked during ship (resolved the PRD open questions)

- Embedding floor = trimmed char count `< 10` (char-aware via `chars().count()`, not byte `len()`). Const `EMBED_MIN_CHARS` in `embed.rs`.
- Embed text = `title + "\n" + content`, applied uniformly to notes (the floor still checks `content` only, so an auto-title never lifts a junk note over the floor). Attachments keep content-only embed text; only their floor changed.
- Migration pass = `embed_missing_notes` runs every boot inside the existing reconcile thread, after `backfill_legacy_chunks`, before `reconcile_once`. Bounded to zero-chunk notes that pass the floor. No network / no consent -> returns 0 and retries next launch. Not flag-guarded (idempotent, self-converging).
