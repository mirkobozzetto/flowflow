# Issue #179 — shared plus popover

Status: implemented; Mirko approved the design after the iPhone installation.
Full haptic/edge-case checklist is not individually confirmed.
Branch: issue/179-plus-popover. No issue closed.

## Scope and decisions
- Read #179 and all comments, roadmap, and matching local/versioned v3 mockup.
- Shared ToolsMenu shell/body for chat and note; anchored to existing + wrappers.
- Root counts, sliding agent/connector panes, visible-scrollbar CSS, settings
  link, Exa tile, chat-only web switch and @ hint. No invented connector rows.
- Note actions still run on the note; chat actions only prefill the launch command.
- No composer, RAG, OAuth or settings-screen changes.
- haptic(kind) did not exist yet (#178 pending): added only selection feedback,
  UISelectionFeedbackGenerator on iOS and no-op elsewhere.
- Exa mark path matches https://exa.ai/images/logo/exa-logo-blue.svg;
  white fill and 0 0 107 129 viewBox match the approved asset.
- @ hint omitted on notes: the existing note bar does not support mentions.

## Verification
- GitNexus refreshed; ToolsMenu/NoteToolsMenu/body/Switch/ConnectorsSection
  impact UNKNOWN (Dioxus RSX callers not resolved). Text search confirmed the
  chat input and recording bar entry points; mention palette constants preserved.
- Full pre-commit detect_changes (limit 500): HIGH for combined prior work;
  no partial/truncated result. Formatting-only chain changes included in scope.
- TypeScript check and transpilation passed through make desktop-build/make all.
- make desktop-build passed; signed macOS app verified with codesign.
- CARGO_BUILD_JOBS=8 make all passed and installed on iPhone de Mirko.
  Log: /tmp/flowflow-179-ios-build.log. Share/widget signing completed.
- Disposable /tmp/flowflow-179-check.mjs passes: viewport fitting, pane focus,
  return focus, unmount cleanup, exact official Exa path.
- Impeccable detector: two pre-existing easing warnings outside this menu;
  no new finding. Requested overshoot remains intentional.
- No on-device visual/haptic claim: Mirko must complete the checklist below.

## Acceptance checklist (iPhone, then Mac chat and note)
- + opens above its button, no veil/sheet; thread remains visible, + rotates.
- Agents and Connectors show real counts; columns align at 28px.
- Chat alone shows web/Exa and @; note has no web/tools section.
- Open each pane and return: horizontal slide/fade, no clipped rows.
- Long connector list scrolls with thin indicator; last row opens Connections.
- Chat agent only prefills; note agent keeps its existing run behavior.
- Outside tap/Escape closes; reopening always shows root.
- iPhone keyboard open and small/landscape viewport: menu stays reachable.
- Enter Connectors, back, toggle web: exactly three selection ticks on iPhone;
  scrolling/connector rows and all Mac interactions have none.
- Empty agents, unavailable backend, missing Exa key: truthful messages.
- Reduced motion: immediate transitions; keyboard focus follows active pane.

## Delivery
User requested preserving/committing/pushing prior work as separate changes.
Prior diff reviewed: account design artifacts, GitNexus guidance, formatting,
Dioxus 0.8 alpha + UIScene migration, GitHub connector icon.
Build verification covers the combined tree, not a runtime migration audit.
Mirko requested a PR into dev and its merge, preserving all remaining work.
No issue closure requested; #179 remains open for any unconfirmed acceptance.

# Issue #178 — shared composer and voice capsule

Status: merged into dev (PR #181, merge 48a911a). Issue #178 still OPEN:
`Closes #178` only auto-closes from the default branch (main), not from dev.
- `src/ui/composer.rs`: `Composer { role }` mounted by chat (`SendMessage`) and
  note (`AppendToNote`); `bar.rs` and `chat_input.rs` deleted.
- `VoiceCapsule` replaces `RecordingControls`: X cancels (replaces double-tap),
  square = dictation for review in the field, arrow = chat: transcribe+send,
  note: keep the clip + durable TranscriptionManager job (unchanged path).
- ponytail: on a note the square path discards the audio clip (dictation file);
  keeping the clip on review needs a transcript↔audio binding, later if wanted.
- Haptics: `haptic`/`haptic_prepare` (medium mic, light stop, soft send, none X).
- Verified: cargo clippy desktop clean on touched files, make desktop-build,
  make all (installed). Transcription/audio storage/TranscriptionManager untouched.

## What broke and why (read this before touching the waveform)
- `document::eval` runs the script as the body of ONE async function and closes
  the Rust->JS channel as soon as that body's promise resolves. An IIFE resolves
  immediately, so `dioxus.recv()` never receives anything. `voice_timeline.js`
  must stay a BARE body with a top-level `await` loop, exactly like
  packages/document/docs/eval.md. It is hand-written JS on purpose: TypeScript
  refuses top-level await outside a module.
- Tailwind does not scan `.js`, so bars created by the script need their rule in
  `tailwind.css` (`.voice-bars i`). The edited file is the SOURCE `tailwind.css`;
  `assets/tailwind.css` is the compiled output dx actually loads.
- An effect that reads a render-local bool subscribes to nothing and runs once at
  mount: `voice_in`/`was_live` read `app.recording_state` directly. And an effect
  must never subscribe to a signal it writes: use `peek()`.
- Debug-only bench: `FLOWFLOW_VOICE_PROBE=1 ./flowflow` (desktop) starts a take
  by itself and logs `[voice] timeline:bars=... w=... tall=...`. A webview
  launched from a shell reports `visibilityState: hidden`, and WebKit then
  FREEZES its animation clock: CSS transitions read as stuck at their start
  value there. Judge transitions in a visible window only.

## Composer shape (validated by Mirko)
Pill on one line, card from the second: the autosize script sets the CAPSULE
height, CSS transitions height and radius (260 ms), buttons stay anchored to its
bottom, the field slides to full width, and the inner focus ring is gone.
Reference: `~/ff-ux-mockup/composer-multiline.html`.

## Open decisions (#178)
- Waveform pacing: 30 ms slice / 4 px pitch today (~133 px/s, felt fast).
  Proposed 50 ms x 3 px (~60 px/s, ~5 s visible) or 40 ms x 3 px.
- Sensitivity: dB range -50..-10 instead of -55..-5, `level^0.7` curve, 1 px floor.
- On a note, the square (review) path keeps no audio clip; the arrow keeps it.
- Not confirmed on device: three haptic ticks, reduced motion, 2 min take.

## Next
#177 (sidebar card + burger) closes phase 1. Then phase 2 starts with
marketplace-flowflow #113 palier 0, no code. Order and prompts:
`.redesign/mobile-ux-2026-09-19/roadmap.html`.
