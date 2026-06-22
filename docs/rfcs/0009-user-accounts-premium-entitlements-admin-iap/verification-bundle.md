# Verification - RFC 0009 Q1.2c + Q1.6

Status of automated checks (run by Claude):
- backend `cargo build` + `cargo test`: 34 passed (incl. new `account_view_reports_members_and_premium`)
- backend `cargo clippy`: clean
- app `cargo check --features desktop`: clean
- app iOS cross-compile `cargo build --features mobile --target aarch64-apple-ios`: clean
- app `cargo clippy` (desktop): clean
- app tests (sync + db + new `account_wipe`): 36 passed

Device install was skipped: the iPhone shows `unavailable` in `devicectl`. Reconnect it, then Claude runs `make all`.

## Step 0 - deploy the backend FIRST (blocking for Q1.6)
`GET /v1/account` is a NEW route. The app account screen calls it, so api.flowflow.be must be redeployed before the screen shows anything.
- Deploy `marketplace-flowflow` to Dokploy.
- Smoke test (any registered device session token):
  `curl -s https://api.flowflow.be/v1/account -H "Authorization: Bearer <session>"`
  -> JSON `{ account_id, premium, device_cap, devices[] }`.

## Step 1 - account screen (Q1.6)
On the iPhone, after `make all`:
1. Settings -> Account (new row, just under General).
2. Confirm it shows: account ID, a Premium/Free badge matching reality, `N / 3` devices, and the device list with "This device" tagged on yours.
3. Tap "Leave account" -> confirm -> the screen reloads to a fresh solo (Free) account with 1 device. Your local notes are still there.

## Step 2 - join over the Noise channel (Q1.2c)
Needs two devices both pointed at api.flowflow.be (Settings -> Connections -> backend URL baked or set).
1. On device A (the one to KEEP its account / premium): Settings -> Sync -> "Show code".
2. On device B: scan / paste the code -> "Connect".
3. Pairing succeeds as before (RFC 0004). Then on device B open Settings -> Account:
   - its account ID now equals device A's account ID,
   - device count is 2 / 3,
   - if A was premium, B shows Premium too.
Rule to remember: the device that SHOWS the code keeps its account; the device that SCANS folds into it.

Edge checks:
- Pair a device with no backend configured -> pairing still works, no account change, no crash (best-effort).
- 4th device join -> backend returns 409; pairing still completes, account just stays unbound (logged).

## Step 3 - delete my data (Q1.6, destructive)
1. Settings -> Account -> "Delete my data" -> confirm the red warning.
2. App returns to the notes list, now empty (notes, chats, attachments, audio, embeddings all gone).
3. Settings still has your keys/backend URL; the device works as a fresh solo install.
