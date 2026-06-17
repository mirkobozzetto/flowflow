Everything shipped on `development` since the backup PR: multi-device sync (RFC 0004), backup/restore (RFC 0001), local Whisper transcription (RFC 0005), a real native desktop experience, a unified design system, and the distributable macOS release pipeline. 269 tests, clippy clean, validated on iPhone + Mac throughout.

## Multi-device sync (RFC 0004)

- 9475668 SQLite change-tracking foundation (version vectors, tombstones, V10)
- d3dcca5 Noise XXpsk3 + QR/IP pairing spikes
- 470a1d8 deterministic chunk ids + vector BLOBs in SQLite
- 3ae68cf vector backfill, reconstruct-from-blob, boot reconcile
- f3dccd1 Noise TCP transport + device pairing
- 28b4a1b at-rest protection + HELLO/PUSH/ACK protocol
- dd2bdf1 conflict merge policy + sync triggers
- e963f20 tombstone GC, add-wins-resurrect, full-state reconcile, reminders sync
- 471e4d1 real-time UI refresh (< 1 s), safe-area fixes, QR deep-link pairing (#22, #23, #24)

## Backup & restore (RFC 0001)

- 06f39aa export, validated import and crash-safe atomic restore with sync reconvergence (#33)
- 61cef04 regression coverage for the crash-safe legacy migration (#27)
- a125ebd PRD marked built (PR #35 follow-up)

## Local Whisper transcription (RFC 0005, #30)

- 76a689b RFC draft, 76aee73 pluggable STT providers + 5-model ggml catalog (sha256-verified downloads, offline transcription, bench)
- 790ce66 detached background downloads + localized EN/FR error reporting (inline transcribe errors, no more silent failures)

## Native desktop experience

- 4f785b5 desktop build without the iOS widget + durable data dir (#20)
- 08cd108 responsive layout pass, permanent sidebar, reading widths (#25)
- a18a844 stable bundle id + dev signature (#34)
- 1515c08 macOS platform layer: microphone/calendar/Apple Events plists, afplay playback, Calendar deep-link, native document/audio imports, shared Apple parsers (PDFKit OCR on macOS)
- ec83db6 keyboard shortcuts (Cmd+N/F/comma, Esc, double-Esc home), view history (Cmd+Left/Right), anchored menus, hover states, desktop scrollbars, text-selection lockdown
- 46e719d instant page switches on desktop + shortcut fallbacks (Cmd+Cmd, Cmd+1/2, Shift+Ctrl+Enter)
- 5264877 `make dmg` / `make release`: distributable DMG + GitHub release pipeline (Developer ID-ready)

## Chat & notes UX

- f171857 per-conversation theme scope, persisted and restored (RAG searches the right theme when reopening an old chat)
- 6a60333 / 835d488 / 9d35462 settings restructured into a grouped sub-screens hub (#31)
- 31b30cf FAB redesign, eb1056a incremental notes list rendering on scroll (30 + 30, smooth past hundreds of notes)

## Design system

- 1240fc6 unified kit: shared button/menu/input constants, confirm-before-delete everywhere (notes included), inline rename with Enter/Esc, vermilion danger palette (oklch, replaces #ff3b30), focus rings on all inputs, 44px touch targets, z-index scale, radius and typography normalization

## Refactors, docs, hygiene

- 29e98a4 detail.rs split into dates/reminders/audio_section (#21)
- 82078bb chunker test alignment
- 07032a3 slim README + chaptered docs index, 48d6ea2 backup docs, b2ac03f roadmap reconciliation
- 2661aaf personal contact details scrubbed from deploy docs
- f4e2958 README refresh: Download section (DMG + App Store), keyboard-first Mac app, updated index
- 8623324 / ba00893 / 02eb4d7 / 3674803 / dd5ea29 agent guides + GitNexus index refreshes

## Verification & housekeeping

- `cargo test`: 269 passed (31 suites); `make check`: fmt + clippy clean
- Device-validated interactively on iPhone and Mac during development
- First public desktop build published: [v0.1.0 release](https://github.com/mirkobozzetto/flowflow/releases/tag/v0.1.0) (DMG)
- Closes #20, #25, #30, #31, #34, #36 (resolutions commented on each); #32 (App Store update) and the lan-serve/agentic-tools half of #33 intentionally remain open
