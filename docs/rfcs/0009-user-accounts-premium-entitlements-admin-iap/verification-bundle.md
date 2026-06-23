# Verification - RFC 0009 Q1.7 cutover (retire PREMIUM_PUBKEYS + drop devices.premium)

Automated checks (run by Claude, marketplace-flowflow on host):
- `cargo fmt`: clean
- `cargo clippy --all-targets`: clean
- `cargo test`: 32 passed (incl. the V4 drop guard `migration_baseline_stamps_without_clobber`)

This release is DESTRUCTIVE: the V4 migration runs `DROP COLUMN devices.premium`
on the prod DB on first boot, forward-only. The code no longer reads
`PREMIUM_PUBKEYS`. So once you deploy, the env bridge and the column are gone for
good. Do the pre-deploy gate first - it is the difference between a clean cutover
and the Mac (premium today only via the #64 allowlist) silently dropping to Free.

Env for the curls below (you fill these):
```
BASE=https://api.flowflow.be
ADMIN_TOKEN=<the prod admin token>
MAC_DEVICE=<the Mac device pubkey, b64 - shown in app Settings -> Connections>
```

## GATE A - prove the entitlement path carries Mirko's premium (BEFORE deploy)
The env still masks everything pre-deploy, so do NOT trust GET /v1/account here;
check the entitlement row directly via the admin API.

1. Admin login -> session cookie + csrf:
```
curl -s -c cj.txt -X POST $BASE/v1/admin/login \
  -H 'content-type: application/json' -d "{\"token\":\"$ADMIN_TOKEN\"}"
# copy the "csrf" value -> CSRF=...
```
2. Find the Mac's account_id (device -> account map):
```
curl -s -b cj.txt $BASE/v1/admin/devices | jq '.[] | select(.device_id=="'"$MAC_DEVICE"'")'
# -> note .account_id -> ACC=...
```
3. Confirm an ACTIVE entitlement on that account (this is what premium rides
   post-cutover):
```
curl -s -b cj.txt "$BASE/v1/admin/entitlements?account_id=$ACC" | jq '.[] | select(.status=="active")'
```
   - If it returns an active row -> GATE A PASSED, go to Step B.
   - If empty -> grant it, then re-check:
```
curl -s -b cj.txt -X POST $BASE/v1/admin/entitlements/grant \
  -H "x-csrf-token: $CSRF" -H 'content-type: application/json' \
  -d "{\"device_pubkey\":\"$MAC_DEVICE\"}"
```
Do NOT deploy until step 3 shows an active entitlement.

## GATE B - backup the prod DB (rollback is restore-only; V4 is forward-only)
Snapshot `app.db` (Dokploy volume) before deploy. If anything goes wrong after,
the only rollback for the dropped column is: redeploy the previous image AND
restore this backup.

## Step C - deploy
Deploy marketplace-flowflow to Dokploy. On boot, migrate() runs V4 (drops the
column). Confirm health:
```
curl -s $BASE/healthz   # -> ok
```

## Step D - prove premium survived the cutover (AFTER deploy)
Now the env is gone, so premium is purely the entitlement. From the Mac
(authenticated session):
```
curl -s $BASE/v1/account -H "Authorization: Bearer <mac session token>" | jq '.premium'
# -> true
```
And a premium-gated connector route still works (the §12 C-cut):
```
curl -s -o /dev/null -w '%{http_code}\n' -X POST \
  $BASE/v1/connectors/google/authorize -H "Authorization: Bearer <mac session token>"
# -> 200 (not 403)
```
- Both green -> cutover done. You can drop `PREMIUM_PUBKEYS` from the Dokploy env
  (the code already ignores it; this is just cleanup).
- `premium:false` or `403` -> ROLLBACK: redeploy the previous image + restore the
  GATE B backup, then re-open the gate (the entitlement was not actually live).

## Step E - app side (no code change, just confirm nothing regressed)
On the iPhone/Mac app: Settings -> Account still shows the right Premium/Free
badge, and Settings -> Connections still lists Google Sheets for the premium
account. No new build is required for Q1.7 (backend-only), but a smoke pass
confirms the gate and the catalog agree.
