---
artifact: "docs/rfcs/0005-pluggable-transcription-providers-with-local-whisper-models/RFC.md"
artifact_kind: "rfc"
engine_tier: "solo"
stepsCompleted: [0, 1, 2, 3, 4, 5, 6]
final_status: "shipped (pending device validation T01/T10, nothing committed)"
updated: "2026-06-11"
---

# Trace Ledger: Pluggable transcription providers with local Whisper models (RFC 0005)

> Single source of truth for run progress. The Accepted RFC.md is never mutated by ship.

## Run setup

- Gate: RFC status Accepted 2026-06-11 (Q1/Q3/Q4/Q5 decided by Mirko via AskUserQuestion; Q2/Q6 owned by T01 device bench = merge gate).
- Engine: solo. Stack: Rust 1.94.1 / cargo / Makefile.
- Q1 decision reshaped T01: user-choice 5-model catalog removed the bench-to-default dependency, so implementation ran in parallel; T01 stays the MERGE gate.

## Task ledger

| ID | Title | Status | Evidence |
|----|-------|--------|----------|
| T01 | Bench harness (MERGE gate) | shipped, device run = Mirko | whisper::bench (time + peak RSS via getrusage) + debug-only UI section in Settings; bundle step 1 |
| T02 | Model manager service | done | models.rs: 5-model catalog, sha256 pinned (HF LFS OIDs verified 2026-06-11), .part+sha256+rename, disk guardrail 2x, single download slot; 7 unit tests |
| T03 | Whisper backend | done | whisper.rs: whisper-rs 0.16, spawn_blocking, Semaphore(1), WAV i16/i32/f32 + N-channel mixdown + linear resample 16k; 5 unit tests + real-model test (tiny, 350 ms, FR transcript OK) |
| T04 | Facade + settings keys | done | provider.rs: SttProvider (FromStr/Display/Default) + TranscriptionClient::{from_db, whisper_from_db, provider, transcribe}; STT_PROVIDER_KEY/WHISPER_MODEL_KEY in settings_repo; dispatch chain integration test |
| T05 | Migration V11 | done | schema.rs: pending_transcriptions rebuilt (transcription_id nullable, provider DEFAULT 'soniox', file_path); repo + 6 tests incl. old-row default + local roundtrip |
| T06 | Swap call sites | done | recording/controls.rs + notes/audio_section.rs: TranscriptionClient::from_db (1-line swaps) |
| T07 | Job manager local pipeline | done | transcription_manager.rs: Job.provider, process_front dispatch, process_local (pending row, select! Polling ticks, resume re-runs from file with output_dir fallback); Soniox pipeline verbatim in process_soniox |
| T08 | Settings UI | done | transcription.rs: provider picker (GeneralSettings 2-button pattern), model cards (absent/downloading %/downloaded, size-confirm Q4, delete, set active), auto-activate first download; 26 i18n keys en+fr |
| T09 | Backup exclusion + docs | done | export includes only DB-referenced WAVs by construction; test archive_never_contains_whisper_model_files (+ settings travel assert); README + CLAUDE.md updated |
| T10 | E2E device validation | PENDING Mirko | verification-bundle.md steps 1-6; build installed on iPhone via make all (exit 0) |

## Self-check results

- make format + make check: clean, 0 clippy warning.
- cargo test (desktop): 268 passed, 11 ignored (249 -> 268, +19).
- iOS cross-compile: aarch64-apple-ios + sim OK with whisper-rs/Metal (IPHONEOS_DEPLOYMENT_TARGET=16.0).
- Real transcription: tiny on FR WAV = 350 ms, correct text; downloaded sha256 == pinned catalog value.
- make all: built, signed, installed on device.

## Deviations from RFC

- T01 reframed from throwaway-branch spike to in-app debug-gated bench harness (consequence of Q1 user-choice decision, recorded in RFC section 8 before acceptance).
- Catalog extended from 3 to 5 models per Q1 (medium-q5_0, large-v3-turbo-q5_0 added).
- whisper-downloaded i18n key shipped unused (card shows Active/Use instead); harmless.

## Next

1. Mirko runs verification-bundle.md (bench T01 on #30, airplane E2E, Soniox non-regression).
2. Commit + push on development after approval (git-guard).
3. After acceptable T01 numbers: merge path + close #30.
