---
artifact: inline (chip removal + themes UI drafts + 1A/2B implementation + scrollbar fix, 2026-06-12)
kind: inline
engine_tier: solo
stepsCompleted: [0, 1, 2, 3, 4, 5]
final_status: shipped (pending device validation by Mirko)
---

# Trace - themes-ui-drafts

| ID | Task | Status | Notes |
|----|------|--------|-------|
| T01 | Remove chat scope chip (redundant with top bar title) | done | chip + chat-scope-label keys removed; per-conversation persistence kept |
| T02 | HTML drafts (3 variants x 2 zones, interactive) | done | drafts-themes.html, opened in browser + sent; Mirko picked 1A + 2B |
| T03 | Scrollbar at window edge (chat + note detail) | done | scroller full width, content centered via lg:px-[max(1rem,calc((100%-48rem)/2))]; notes list/settings already correct |
| T04 | Zone 1A: + pivots to x, accordion create input, ghost check | done | folders.rs header + new-theme input (always-mounted accordion, eval focus) |
| T05 | Zone 2B: anchored popover menus with labels | done | folders.rs (rename/subtheme/delete red) + conversations.rs (rename/delete); popIn keyframe; backdrop z-20; dots hover-reveal on lg |
| T06 | Subtheme input harmonized | done | soft wrap + ghost check style |
| T07 | Native feel: text selection disabled app-wide except input/textarea | done | user-select none + touch-callout none in base layer |
| T08 | NoteMenu + ChatMenu aligned on popover style | done | popIn + border-stone-200 + p-1 + rounded items + separator + red delete hover |

## Verification

- make check: PASS; cargo test: 269 passed
- make desktop-app + make all: background task b0pepboiw

## Notes

- folder-menu-* i18n keys shared by folders and conversations popovers.
