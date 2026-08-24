---
artifact: "docs/proposals/0001-person-profiles-avatar-identity-public-note-snapshots/PROPOSAL.md"
artifact_kind: "propose"
engine_tier: "solo (wave 0); teams available for post-gate waves"
stepsCompleted: [0, 1, 2, 3, 4]
final_status: "shipped"
updated: "2026-08-23"
resume_cmd: "/ship -r docs/proposals/0001-person-profiles-avatar-identity-public-note-snapshots/PROPOSAL.md"
---

# Trace Ledger: 0001 person profiles + shared notes/threads

> Single source of truth for progress. A fresh session reads THIS file
> plus contract.md (revision 1) to resume. contract.md OVERRIDES the
> proposal's section 6/10 where they conflict: the scope changed at the
> T01 gate (thread sharing + deletion alignment replaced the frozen
> snapshot model). The validated mockup `mockups/index.html` is the
> visual contract.

## Statut final

Shipped 2026-08-23 (commit dev 930ccc1). UX pass menus incluse.
Issues: #86 commentée, #88 créée (dossiers + lien https universel).

## Halt reason (historique)

Clean handoff requested by Mirko (2026-08-23): T01 gate PASSED
(mockup v2 validated, Q2/Q3 decided), implementation not started.
Resume = execute T03..T18 per contract.md revision 1.

## Tasks

| Unit | Contract item | Status | Files touched | Engine | Notes |
|------|---------------|--------|---------------|--------|-------|
| T01 | Mockup validated, Q3 decided | done | `mockups/index.html` | solo | v2 validated by Mirko; app design system, no code on share card, avatar via tap, threads added; radius/overflow fixes applied |
| T02 | Prod deploy proven | done | none | solo | 302 -> /login 200; fix a1429c5 on origin/main |
| T03 | Migration V17 (profile + shares tables) | done | marketplace: src/db/migrations.rs, tests/migration_v17.rs | solo | 5 tables, author SET NULL, cascades tested; branch feat/profiles-shares |
| T04 | Backend features/profile web routes | done | marketplace: src/features/profile/{mod,fields,avatar,repo,routes}.rs, Cargo.toml (image), src/error.rs (NotFound), tests/profile_test.rs | solo | KV upsert + validation, avatar base64->JPEG re-encode (EXIF out) |
| T05 | Device plane profile + avatar_hash | done | src/features/profile/routes.rs, src/lib.rs | solo | groups+public + avatar_hash; unlinked = 404 |
| T06 | Backend features/shares (publish/read/append/delete/revoke/purge) | done | marketplace: src/features/shares/{mod,repo,routes}.rs, src/lib.rs, src/db/mod.rs, tests/shares_test.rs | solo | codes en body (hors logs), 404 uniforme, tombstones, purge dans sweep |
| T07 | Lifecycle: delete-my-notes, account ordering, reports | done | src/features/accounts/routes.rs, src/features/shares/ | solo | dissolve_account_content sur leave + join-fold; report dédupliqué |
| T08 | Rate-limit bucket share-read plane | done | src/ratelimit.rs, src/state.rs, src/main.rs | solo | seau /v1/shares dédié, test flood vs auth |
| T09 | Admin moderation (reports, revoke unit/global) | done | marketplace: src/features/admin/shares.rs, src/lib.rs; admin/src: features/dashboard/{api,queries,i18n}.ts, components/{moderation-screen,dashboard-tabs}.tsx, routes/_app/_admin/dashboard.moderation.tsx | solo | lecture auditée (mutation on-click), revoke-all armé 2 clics, EN/FR; build+typecheck+lint OK |
| T10 | Site Profile pane | done | account: components/{Dashboard,Profile}.astro, lib/{api,proxy}.ts, scripts/profile.ts, styles/beta.css (b-pfield: b-field était pris), i18n EN/FR, pages/index + fr | solo | build + astro check verts; screenshots envoyés à Mirko (preview 4399); proxy fetch-fail -> 502 |
| T11 | Site avatar photo | done | account: components/Avatar.astro (photo + repli onerror) | solo | GET /v1/me/profile/avatar via proxy /v1/me |
| T12 | App image display spike | done | décision | solo | data URI (avatar re-encodé ~<=60 Ko); à confirmer on-device au T14/T18 |
| T13 | App V25 + typed client | done | flowflow: schema.rs V25 (note_shares, note_provenance, PK id), sync_meta.rs, sync/protocol/catalog.rs, persistence/{mod,share_repo,settings_repo,note_repo}.rs, backend/{mod,profile,shares}.rs, application/{sharing,profile,note_persistence,mod}.rs, domain/share.rs | solo | V24 app était pris (sync_peers.name) -> V25; sync+backup inclus; lien flowflow://share/{code} |
| T14 | App account-card avatar | done | src/ui/settings/account.rs, application/profile.rs | solo | validé iPhone (photo visible) |
| T15 | App share note/thread + share card | done | share_section.rs, menus note/fil/ligne, state.rs | solo | Partager dans les 3 menus: publie + copie + scroll; lien https cliquable reporté (#88) |
| T16 | App open link, append/edit/delete own, keep, deletion alignment | done | ui/shared/{view,open_link}.rs, watchers.rs, router.rs | solo | deep link flowflow://share + entrée tiroir |
| T17 | App Store 1.2 compliance | done | shared/view.rs (report, block), settings/privacy.rs (ToS, contact) | solo | |
| T18 | Deploys + e2e device | done | backend+site+admin déployés (PR #87, #127, #128); app installée iPhone | solo | photo validée device; cycle partage à finir de valider par Mirko |

## Decisions locked (do not re-ask)

- Q3 = app-only v1 (no public web page).
- Q2 = 64 KB/note, 100 active shares/account, attachments excluded.
- Avatar via tap on the photo; no field row.
- Share card: Copy link + Revoke, code never displayed.
- Lifecycle rules 1-5: see contract.md "Shared-content lifecycle".

## Checkpoints

| Step | Kind | Decision | Why |
|------|------|----------|-----|
| step-04 | gate T01 | passed | Mirko validated mockup v2 (round 1 fixes applied) |
| step-04 | handoff | halted | Mirko: prepare for /clear + fresh /ship resume |

## HALT events

- 2026-08-23: clean halt at the T01/T02 boundary for context reset.
  No code written in any repo. Nothing to revert.

## Resume protocol (fresh session)

1. Read contract.md (revision 1) — it overrides PROPOSAL sections 6/10
   on sharing (threads, deletion alignment, device-auth read plane).
2. Open mockups/index.html for the validated visuals.
3. Execute T03..T18 in DAG order; parallel: T04 with T06, site wave
   with backend wave, T14/T15/T16 between them.
4. Backend tests mold: marketplace tests/ (~250 tests). App gate:
   make check + make all, device validation by Mirko before any push.
5. Issue thread = marketplace-flowflow#86 (comment 5384945530 records
   the scope change).
