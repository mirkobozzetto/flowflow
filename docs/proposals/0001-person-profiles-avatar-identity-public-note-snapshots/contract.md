---
type: contract
source: PROPOSAL.md (0001, status Accepted) + T01 mockup review round 1
locked: 2026-08-23
revised: 2026-08-23 (revision 1, thread sharing)
stack:
  backend: rust/cargo (marketplace-flowflow)
  site: astro/bun (flowflow/account)
  app: rust/cargo + make (flowflow, iOS Dioxus)
---

# Definition of done — 0001 person profiles + shared notes/threads

Revision 1 supersedes the frozen-snapshot model of the proposal:
sharing covers single notes AND app threads, with backend-mediated
deletion alignment. The mockup (`mockups/index.html`, validated by
Mirko 2026-08-23) is the visual contract.

## Decisions locked at the T01 gate

- Q3: app-only in v1. No public web page; a share link opens only in
  FlowFlow. Reading therefore happens on the authenticated device
  plane (Ed25519), there is NO unauthenticated content plane.
- Q2: 64 KB text max per shared note; 100 active shares max per
  account; attachments excluded from v1. Server-side constants.
- Avatar: changed by tapping the avatar (camera badge). No identity
  field row, no visibility pill for the photo; it travels with the
  display name on shared content.
- Share card: never shows the code. Actions = Copy link / Revoke.
  No new typography.
- Phone UI follows the app design system: system font, rounded-xl
  (12px) cards, stone-200 borders, warm-white bg, thread timeline
  (orange dot + line), pill buttons border ios-orange/25.
- i18n EN/FR everywhere from day one (site account.json, app t()).

## Shared-content lifecycle rules (from Mirko, round 1)

1. A shared thread is multi-author: anyone who opened the link with a
   linked account can append their own notes to the fil.
2. Each participant edits/deletes ONLY their own notes.
3. Deleting one's note emits a deletion signal via the backend: every
   phone removes the note AND its embeddings from the local knowledge
   base; the fil shows a tombstone ("note deleted by its author").
4. Deleting the account does NOT delete its shared notes.
5. Explicit "delete my notes" removes them everywhere, on all reader
   phones; it must run BEFORE account deletion.

## Acceptance criteria (revision 1)

| ID | Criterion |
| --- | --- |
| T01 | DONE — mockup v2 validated by Mirko (2026-08-23), Q3 decided app-only |
| T02 | DONE — account.flowflow.be answers in prod, runtime fix a1429c5 on origin/main |
| T03 | V17 migration passes on an existing DB: profile tables (web_user_profile_fields KV + web_user_avatars) + shared_threads + shared_notes (author, tombstone deleted_at) + share_reports; FK cascades tested |
| T04 | GET/PUT profile fields, GET/PUT/DELETE avatar; 256 KB rejected; magic bytes; re-encode + strip EXIF; social URLs validated; tests |
| T05 | Linked device receives groups/public fields + avatar_hash; unlinked = clean 404 |
| T06 | Backend shares: publish note/thread (linked web_user + premium + quota 100/64 KB, expiry mandatory), read by code (device auth), append own note, edit/delete own note (tombstone signal), revoke, purge expired; indistinguishable 404; codes out of logs; tests |
| T07 | Lifecycle: "delete my notes" purges own shared notes everywhere; account deletion keeps shared notes but requires the purge path to exist first; leave revokes account shares, purges profile+avatar, cleans web_user_accounts; authenticated deduplicated report; tests |
| T08 | Dedicated rate-limit bucket for the share-read plane; auth unaffected; test |
| T09 | Admin: reported-share list, audited content read, unit + global revoke, i18n |
| T10 | Site Profile pane per mockup section 1: fields + pills (groups inert "soon"), avatar tap-to-change, CSRF, EN/FR |
| T11 | Site avatar photo via GET avatar, bounded upload, monogram fallback |
| T12 | Spike: downloaded image displays in iOS webview; approach decided |
| T13 | App V24: share codes, kept-content provenance + alignment state, backup + sync included; typed client profile + shares |
| T14 | Photo replaces monogram on account card per mockup section 2; hash cache; offline fallback; visible link path on 404 |
| T15 | Share a note/thread per mockup section 3: share card (Copy link / Revoke, no code shown); reshare replaces; local delete offers revoke |
| T16 | Open a link per mockup sections 3-4: read-only view, append own note in a shared thread, edit/delete own notes only, keep with provenance, deletion signal removes note + embeddings with tombstone + banner |
| T17 | App Store 1.2: in-app report, author block, ToS + contact, privacy labels |
| T18 | Backend + site deployed; issue #86 criteria pass on iPhone, including the thread deletion-alignment scenario |

## Out of scope (never build)

- E2e profile encryption (v2)
- Social graph (follow, friends, feed, profile discovery)
- Moderation beyond report + admin revoke
- Shared folders (sibling RFC)
- Public web reading page (postponed, was Q3 option A)
- Snapshot password
- Attachments in shared content (v1)

## Edit scope (authorized files)

Backend (marketplace-flowflow): src/features/profile/ (new),
src/features/shares/ (new, replaces planned features/snapshots/),
src/db/migrations.rs, src/lib.rs, src/ratelimit.rs,
src/features/accounts/routes.rs, src/features/admin/ + admin/src/,
Cargo.toml, tests/.

Site (flowflow/account): src/components/Dashboard.astro,
Profile.astro (new), Avatar.astro, src/lib/api.ts, src/scripts/,
src/i18n/*/account.json.

App (flowflow): src/infrastructure/persistence/,
src/infrastructure/backend/, src/ui/settings/account.rs,
src/ui/notes/detail/, src/ui/thread/, src/ui/ + state.rs +
app/router.rs.

NOT in scope: note_card.rs author chips (unchanged in v1).

## Verification commands (per repo)

- Backend: `cargo test` in marketplace-flowflow
- Site: `bun run build` in flowflow/account (visual check vs mockup)
- App: `make check` + `make all`; T18 = manual iPhone protocol
