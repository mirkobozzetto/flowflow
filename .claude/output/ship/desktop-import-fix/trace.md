---
artifact: inline (desktop imports + rename style + copy note, 2026-06-12)
kind: inline
engine_tier: solo
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: shipped (pending device validation by Mirko)
---

# Trace - desktop-import-fix

| ID | Task | Status | Notes |
|----|------|--------|-------|
| T01 | Desktop "Importer un document" works | done | parsers.rs + pdf.rs moved to platform/ (shared ios+macos), PDFKit enabled for macOS target (objc2/foundation/pdf-kit deps added to macos cfg), rfd::FileDialog picker (same pattern as backup restore) |
| T02 | Desktop "Importer un audio" works | done | rfd picker filter m4a/mp3/wav/aac/caf/aiff/flac |
| T03 | Sidebar rename rows restyled | done | soft stone-100 wrap + ghost check + X, popIn, Esc cancels (conversations + folders) |
| T04 | Esc cancels create inputs | done | new theme + subtheme inputs |
| T05 | Copy note menu item | done | NoteMenu "Copier la note" (title + content via clipboard helper), i18n EN/FR |

## Diagnosis (no code change)

iPhone picker stuck on "Récents / CHARGEMENT": system Files picker (out-of-process) querying iCloud Drive; fileproviderd stuck or iCloud unreachable. App only presents the picker. Checks: Files app Récents spins too -> reboot/kill Files; Explorer > Sur mon iPhone works locally. Optional app-side mitigation: set picker directoryURL to a local folder (not implemented).

## Checkpoints

- New target deps for macOS: objc2, objc2-foundation, objc2-pdf-kit (same crates as iOS, PDFKit exists on macOS). Logged per ship guardrails (auto mode, non-destructive).
