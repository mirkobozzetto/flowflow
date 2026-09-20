---
proposal_id: "0010"
slug: "account-home-redesign"
title: "A profile-first FlowFlow account"
status: Accepted
format: short
created: "2026-09-10"
---
# 0010: A profile-first FlowFlow account

## 1. Decision
Propose one useful home page, with Subscription and Security as separate tabs.
Review `draft.html` before implementing. It is a standalone French prototype
with fictional data, embedded existing brand assets and no backend calls.
Issues: #156, #157, #158, #164, #165. #166 remains research only.

## 5. Alternatives considered
- Keep the dashboard: preserves duplicate views and backend-shaped labels.
- Copy the entire native settings UI: imports local-only assumptions into web.
- Recommended: reuse its visual rows and supported contracts, not local state.

## 6. Proposed design
Home: full editable profile and photo controls, named devices, people/agents
within shared spaces. Remove standalone Devices/Services tabs, raw provider
rows, empty requests, origin/granted labels and the hard-coded device cap.
Subscription: actual resolved Premium state and expiry only when applicable;
no invented price or unlimited claim. Security retains the existing login
history without misrepresenting it as a live-session registry.
Setup: visible logo, French copy, email verification and explicit device-link
instructions. Delivery errors must be actionable; no fake success response.

### Native reuse and boundaries
- `src/ui/settings/account.rs`: reuse the identity card and compact device rows.
- `src/ui/icons.rs`: phone/laptop SVG geometry copied into the HTML prototype.
- `src/application/device_naming.rs`: preserve the stored name; generated names
  remain valid user identities. Do not fabricate a platform from a device id.
- `device_label` uses local device/peer knowledge unavailable in the browser.
  Web must prefer backend name and use a localized neutral fallback if absent.
- Native sibling icon selection is heuristic. Web needs explicit platform data
  or a neutral icon, not the inverse of the current browser's platform.
- `SyncPairingView` pushes names to the backend on blur. Audit the web endpoint
  projection to reuse these names rather than adding a second naming system.
- Native pairing QR hosts a device-to-device session. It must not be copied
  as a working web QR. Web account-link codes follow the existing link route.
- `last_seen` is last activity, not current connectivity or sync health.
- Use existing profile upload/visibility semantics. Demo initials are fallbacks,
  not scraped personal photos. Hermes uses the existing official image.
- People must be authorized and grouped by space. An authorized Hermes token
  does not establish agent online presence.

## 7. Drawbacks and risks
Names/platform and collaborator projection need backend verification. Render
loading, unavailable and empty states separately. Do not equate API failures
with no devices. Preserve privacy and existing profile visibility choices.
The HTML demonstrates layout and interactions, not completed email delivery,
authentication, uploads, revocation or production i18n.
Future server-backed shared folders/messages should be scoped separately from
personal local-first chat (#166). Discuss the boundary before creating another
issue; no web messaging or new issue is included in this draft.

## 10. Implementation plan
1. Accept or revise the HTML layout and navigation with Mirko.
2. Fix #164: reproduce mail failure; preserve locale through auth; reuse logo.
3. Resolve #157 data projection: names, explicit platform/fallback, last activity.
4. Implement #156/#158 using existing Astro tokens, profile and resolved access.
5. Implement #165 only after authorizing the per-space people/agent projection.
6. Verify French/English, mobile navigation, API failure/empty states, real mail
   delivery and device identity consistency before considering delivery complete.

Mirko accepted revision 03 and authorized implementation on dev.
Deployment and live-data changes still require separate authorization.

## Draft revision 02
User review: preserve the FlowFlow design system rather than inventing a
brown palette. Reuse orange/stone colors from `tailwind.css` and button,
input, radius and elevation conventions from `src/ui/kit.rs`. Navigation
and folder SVG paths come directly from `src/ui/icons.rs`; device glyphs
remain the exact native ones. Use normal sans-serif Premium typography,
without a redundant active badge. Never render a free-account preview or
request button inside the Premium account. Free/pending/expired states are
mutually exclusive production states, not extra content for a Premium user.
The mockup does not resolve any GitHub issue by itself.

## Draft revision 03
Restart from the original account CSS and Profile.astro structure. Preserve
original buttons, plain navigation, all seven profile fields, avatar upload
entry point and working private/public controls directly on home. Remove
invented accordions, decorative menu glyphs and reduced-profile modal.
The existing inert Groups/soon control is omitted, not presented as functional.
Photo preview uses the existing 512px JPEG pattern but sends nothing to a server.
This is a cleanup plus device names and collaborators, not a new visual system.
