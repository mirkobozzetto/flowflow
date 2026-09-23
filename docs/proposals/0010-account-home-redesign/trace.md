# Account redesign implementation

Status: in progress. User accepted draft revision 03 and authorized work on dev.
Scope: #156, #157, #158, #164, #165. #166 excluded.

- Confirmed original Astro profile/photo implementation must be retained.
- No code changes yet. Inspect backend projections before defining web types.
- No deployment or production mutations authorized.
- Need focused UI/SSR checks and real-email evidence before issue closure.

## Device-name projection started
- Backend accounts repo/route now includes existing `devices.name` in the
  authenticated `/v1/me/devices` response. Existing account-id filter retained.
- Web DeviceRow accepts optional name for compatibility with old backend.
- No platform column exists; web must use a neutral icon unless explicit data
  becomes available, not infer an iPhone from a public key.
- Existing `cargo test --test account_link_test` started in backend dev;
  inspect hbackendtest on completion. No permanent tests added.
- Index refresh succeeded; MCP still reports cached old freshness. Graph calls
  corroborated with source: one repo caller, HTTP route explicitly mounted.
- Unrelated backend `.redesign/` untracked files are untouched.

## Home implementation started
- Existing backend account-link tests: 5 passed.
- Dashboard now mounts existing Profile directly, keeps Subscription/Security,
  drops raw services/requests/provenance/device cap, and reads resolved Premium
  from onboarding instead of guessing from the first entitlement row.
- Device endpoint failures render unavailable, not an empty list.
- Existing photo upload and all profile fields retained. Existing groups
  visibility preserved where stored, but unavailable Groups choice not promoted.
- Mobile navigation retained; sidebar no longer removes all navigation.
- EN/FR home copy added. Neutral device icon until authoritative platform exists.
- `npm run check` running in account; inspect hwebcheck on completion.
- Collaborator projection, locale routing and mail diagnosis still pending.

## Collaborator access and locale
- Added authenticated collaborator projection and avatar route. Each query checks
  live space and membership; only public names/photos are exposed to other users.
- `cargo test --test account_presence_test --test account_link_test`: 6 passed.
  New test covers private/public profiles, outsider denial and removed membership.
- Added locale preference cookie, browser-language selection and FR/EN switch.
  Onboarding now uses the real logo.
- Production mail cause remains unconfirmed: delivery requires MAIL_DELIVERY_URL,
  ACCOUNT_PORTAL_BASE, MAIL_FROM and optional MAIL_API_KEY. No live call made.
- Web check/build running (hwebfinal). Local fixture preview starting on port
  4322 (hpreview). Preview is not proof of live writes or email delivery.
- User requires visual validation before PR/deployment. No commit or push.

## Local visual checkpoint
- Astro check: 0 errors, warnings or hints. Build completed; server entry.mjs exists.
- Browser checks passed in installed Chrome: FR browser redirect, persistent EN/FR
  choice, three tabs, Premium gating, all seven profile fields and visibility in
  intercepted save payload, successful-save feedback, mobile navigation and no
  horizontal overflow at 390px. No uncaught browser errors.
- Desktop/mobile screenshots inspected at /tmp/flowflow-account-{desktop,mobile}.png.
- Local preview: http://127.0.0.1:4322/fr (fixture data, hpreview remains running).
- API writes were intercepted locally. Actual photo persistence and email delivery
  are not proven by this visual checkpoint. Mail configuration/provider evidence
  is still required for #164; no issues closed, no deployment, commit or PR.

## Visual approval
- User approved the page, requesting only a sharper Hermes PNG.
- Replaced the 48x48 native icon with the existing landing-page 252x256 image.
  Same artwork, no synthesized details. Local preview uses the replacement.

## PR delivery
- User approved PRs targeting main and issue progress comments.
- Main and dev trees matched, but squash history would include previous changes
  in PR diffs. Created dedicated branches from origin/main without rewriting dev.
- Portal commit 43413dc, PR https://github.com/mirkobozzetto/flowflow/pull/167.
- API commit a034eeb, PR https://github.com/mirkobozzetto/marketplace-flowflow/pull/100.
- #156 closes on merge; #157/#158/#164/#165 retain remaining integration checks.
- Untracked design artifacts and unrelated backend work excluded from commits.
- No merge, deployment, production mutation or webhook call.

## Main/dev alignment after user merge
- User merged both PRs and requested main -> dev in both repositories.
- Reviewed and resolved two portal conflicts to the approved main versions;
  backend merged cleanly. Exact staged trees matched origin/main before commit.
- Pushed merge commits: flowflow dev 82d3b8a; marketplace-flowflow dev 45a18e4.
- Verified origin/main is an ancestor of origin/dev and both trees are identical
  in each repository. Untracked artifacts preserved. Both checkouts now on dev.
- User will report runtime results after Dokploy deployment; no webhook called.

## Email/CRM scope simplification
- User decided verification and CRM delivery are pre-launch design work.
- Created #168 for future email/CRM data-flow decisions.
- Removed verification UI and all Premium request/approval gates; account email
  remains stored and existing email mechanisms remain dormant for compatibility.
- Checks: Astro check/build pass; native cargo check pass; 7 backend tests pass;
  browser proves an unverified linked account can request Premium with no email UI.
- Published directly to dev then fast-forwarded main: flowflow f442d85;
  marketplace-flowflow 3a2d0f3. Both repositories returned to dev.
- Closed #157, #158, #164 and #165 with final evidence. #168 remains open.
