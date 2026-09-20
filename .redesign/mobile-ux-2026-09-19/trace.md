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
