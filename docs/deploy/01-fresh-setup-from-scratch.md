# Fresh setup from scratch — iOS signing for FlowFlow

This guide walks a contributor from "I just cloned the repo" to a working `make appstore` IPA. It also covers the simpler `make all` (debug on physical iPhone) and `make dev` (simulator) flows.

Estimated time: ~45 min the first time. Most steps are one-time per Apple Developer account.

## TL;DR — three signing modes

| Mode | Command | Cert needed | Profile needed | Apple plan |
|------|---------|-------------|----------------|------------|
| Simulator | `make dev` | none | none | free |
| Debug on device | `make ddev` / `make all` | Apple Development | iOS App Development | free (7-day) or paid |
| App Store distribution | `make appstore` | Apple Distribution | App Store (× 2) | paid (€99/year) |

`make dev` works for everyone out of the box. `make appstore` only works if you intend to publish to the App Store under your own Apple Developer Program account.

## Prerequisites

- macOS 14+ (Sonoma)
- Xcode Command Line Tools installed: `xcode-select --install`
- Rust toolchain (`rustup default stable`, then `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`)
- `dx` CLI 0.7.9: `cargo install dioxus-cli@0.7.9 --force`
- `ImageMagick` (only for icon manipulation): `brew install imagemagick`
- An Apple ID. For App Store submission only: an Apple Developer Program enrollment ($99/year)

Clone:

```bash
git clone https://github.com/mirkobozzetto/flowflow.git
cd flowflow
```

Copy and fill `.env`:

```bash
cp .env.example .env
# Edit .env, fill SONIOX_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY.
# Only fill APPLE_TEAM_ID if you plan to run `make appstore`.
```

## Path A — Simulator only (zero signing)

```bash
make dev
```

That's it. The iPhone simulator boots and the app runs. No cert, no profile, no Apple account.

## Path B — Debug on physical iPhone (free Apple ID is enough)

This path uses Xcode auto-provisioning (Personal Team). Profiles expire every 7 days on free Apple IDs and every 1 year on paid plans — `make all` auto-renews on expiry.

### One-time setup

1. Enable Developer Mode on the iPhone: Settings → Privacy & Security → Developer Mode → On → restart.
2. Connect via USB, accept "Trust This Computer" on the iPhone.
3. Pair the device: `xcrun devicectl manage pair --device <DEVICE_ID>` (find DEVICE_ID via `xcrun devicectl list devices`).
4. In Xcode, sign in: Xcode → Settings → Accounts → click `+` → Apple ID → enter credentials.
5. Generate the first provisioning profile via the workaround documented in `CLAUDE.md` step 7 ("Physical Device Setup"). Short version: open Xcode, create a throwaway Swift App project with bundle ID `com.<your-name>.flowflow`, hit Cmd+R targeting your iPhone, Xcode creates the profile for you. Delete the throwaway project; the profile stays in `~/Library/Developer/Xcode/UserData/Provisioning Profiles/`.

### Day-to-day

```bash
make all
```

Builds the Rust app, signs the widget, injects the icon, installs on the iPhone. If a profile expired, `make all` auto-renews via `make renew` (Xcode `xcodebuild` headless build of the throwaway project).

## Path C — App Store distribution (paid Developer Program required)

This is the full signing chain needed by `make appstore` to produce an IPA that App Store Connect will accept.

You need, in order:

1. An Apple Distribution **certificate** (one per machine, 1-year validity).
2. Two App Store **provisioning profiles**: one for the main app bundle, one for the widget extension.
3. A correctly populated `.env` with `APPLE_TEAM_ID`.

### Step 1 — Generate the CSR (Certificate Signing Request)

Apple needs a CSR to issue you a distribution certificate. The fastest way to make one is via `openssl`:

```bash
mkdir -p secrets/ios
openssl req -new -newkey rsa:2048 -nodes \
  -keyout secrets/ios/distribution.key \
  -out secrets/ios/distribution.csr \
  -subj "/emailAddress=<your-email>/CN=<Your Full Name>/C=<your-country-code>"
```

- `secrets/ios/distribution.key` is your private key. **Never commit it.** **Never share it.** Back it up to an encrypted location (1Password, an external drive).
- `secrets/ios/distribution.csr` is the public CSR you upload to Apple.

(Alternative: Keychain Access → Certificate Assistant → Request a Certificate From a Certificate Authority. Same result, more clicks.)

### Step 2 — Create the Apple Distribution certificate on Apple Developer Portal

1. Open <https://developer.apple.com/account/resources/certificates/list>
2. Click `+` (Create a New Certificate).
3. Under **Software**, select **Apple Distribution** ("Sign your iOS, iPadOS, macOS, tvOS, watchOS, and visionOS apps for release testing using Ad Hoc distribution or for submission to App Store Connect."). Do NOT pick "iOS Distribution (App Store and Ad Hoc)" — that one is legacy.
4. Continue.
5. Upload your CSR (`secrets/ios/distribution.csr`).
6. Continue → Download. You get `distribution.cer`. Save it to `secrets/ios/distribution.cer`.

### Step 3 — Install the certificate in Keychain

```bash
security import secrets/ios/distribution.cer -k ~/Library/Keychains/login.keychain-db
security import secrets/ios/distribution.key -k ~/Library/Keychains/login.keychain-db -T /usr/bin/codesign
```

Verify:

```bash
security find-identity -v -p codesigning | grep "Apple Distribution"
```

Expected output: one line like `Apple Distribution: Your Name (XXXXXXXXXX)`. The 10-char code in parentheses is your **Team ID**. Note it down — you need it for `.env`.

### Step 4 — Create the App IDs (one-time, if not already present)

Visit <https://developer.apple.com/account/resources/identifiers/list>. You need two App IDs:

- `com.<your-name>.flowflow` (main app)
- `com.<your-name>.flowflow.recording-widget` (Dynamic Island widget extension)

If they already exist (created automatically when you did Path B), skip. Otherwise click `+` → App IDs → App → Continue → Bundle ID Explicit → fill name + bundle ID → Continue → Register.

### Step 5 — Create the two App Store provisioning profiles

Visit <https://developer.apple.com/account/resources/profiles/add>.

**Profile 1 — main app:**

1. Distribution → **App Store Connect** → Continue.
2. App ID → pick the main one (`com.<your-name>.flowflow`) → Continue.
3. Certificates → check the Apple Distribution one you just made → Continue.
4. Provisioning Profile Name: `FlowFlow App Store` → Generate.
5. Download. Move to `secrets/ios/FlowFlow_App_Store.mobileprovision`.

**Profile 2 — widget:**

Same flow, but pick the `recording-widget` App ID and name it `FlowFlow Widget App Store`. Save as `secrets/ios/flowflow_recordingwidget.mobileprovision`.

### Step 6 — Install the two profiles in Xcode's profile directory

`make appstore` reads provisioning profiles from `~/Library/Developer/Xcode/UserData/Provisioning Profiles/`. Install both using their UUIDs as filenames:

```bash
PROFILES_DIR=~/Library/Developer/Xcode/UserData/Provisioning\ Profiles
mkdir -p "$PROFILES_DIR"

for f in secrets/ios/FlowFlow_App_Store.mobileprovision secrets/ios/flowflow_recordingwidget.mobileprovision; do
  UUID=$(security cms -D -i "$f" | plutil -extract UUID raw -)
  cp "$f" "$PROFILES_DIR/$UUID.mobileprovision"
  echo "Installed $f → $UUID.mobileprovision"
done
```

### Step 7 — Fill `APPLE_TEAM_ID` in `.env`

Use the Team ID you noted in Step 3 (the 10-char code from `security find-identity`):

```
APPLE_TEAM_ID=XXXXXXXXXX
```

### Step 8 — Build the IPA

```bash
make appstore
```

Expected end-of-log:

```
>> FlowFlow.ipa ready. Upload via Transporter.app.
```

`FlowFlow.ipa` lands at the repo root.

### Step 9 — Upload to App Store Connect

Two options:

```bash
xcrun altool --upload-app -f FlowFlow.ipa -t ios \
  -u <your-apple-id> -p <app-specific-password>
```

Or drag-and-drop `FlowFlow.ipa` into <https://apps.apple.com/us/app/transporter/id1450874784>.

After upload, the build appears in App Store Connect → My Apps → TestFlight (≤30 min for processing). From there: fill metadata, screenshots, privacy labels, and submit for review.

## Troubleshooting

### `Apple Distribution: no identity found`

You skipped Step 3 (cert install). Double-click `secrets/ios/distribution.cer` to add it to Keychain, then re-run `security find-identity -v -p codesigning` to confirm.

### `Failed to verify code signature of … recording_widget.appex : 0xe800801c`

The widget extension was not signed. This means `make appstore` ran an old version of the Makefile that did not call `scripts/sign-widget.sh release`. Pull the latest `main`.

### `ERROR: no App Store provisioning profile for com.mirkobozzetto.flowflow`

Step 5 / Step 6 incomplete. Verify both profiles are present:

```bash
ls ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/*.mobileprovision
for f in ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/*.mobileprovision; do
  P=$(security cms -D -i "$f" 2>/dev/null)
  NAME=$(echo "$P" | plutil -extract Name raw - 2>/dev/null)
  if echo "$P" | grep -q ProvisionedDevices; then T=dev; else T="App Store"; fi
  echo "[$T] $NAME"
done
```

Both `[App Store]` entries must appear.

### `inject-icon.sh` warning: `No simulator runtime version from … available`

Harmless. `actool` complains because it tries to compile the icon for the simulator too. Ignore.

### `0xe8008011 This provisioning profile has expired.`

You are on a free Apple ID (Personal Team), profiles last 7 days. Run `make renew` to regenerate, then re-run `make all`. Or upgrade to the paid Developer Program ($99/year, 1-year profiles).

### Switching Mac

Your `secrets/ios/distribution.key` is what binds your identity. Copy `secrets/ios/` from the old machine to the new one, then re-import on the new Mac:

```bash
security import secrets/ios/distribution.cer -k ~/Library/Keychains/login.keychain-db
security import secrets/ios/distribution.key -k ~/Library/Keychains/login.keychain-db -T /usr/bin/codesign
# then re-install the two .mobileprovision via the Step 6 snippet
```

If you lost the `.key`, you must revoke the old cert on Apple Developer Portal and start over from Step 1.

## What `secrets/` contains

| File | Purpose | Sensitivity |
|------|---------|-------------|
| `secrets/ios/distribution.key` | Private key for Apple Distribution cert | **CRITICAL** — never commit, never share, back up encrypted |
| `secrets/ios/distribution.csr` | Public CSR sent to Apple | Low (informational) |
| `secrets/ios/distribution.cer` | Apple Distribution certificate | Medium (bound to your Team ID) |
| `secrets/ios/development.cer` | Apple Development certificate (optional) | Medium |
| `secrets/ios/FlowFlow_App_Store.mobileprovision` | Main app distribution profile | Medium |
| `secrets/ios/flowflow_recordingwidget.mobileprovision` | Widget distribution profile | Medium |

`secrets/` is fully `.gitignore`d. Apple revokes certs/profiles cleanly if leaked — no app uploaded with your account can be tampered with by anyone holding only these files (the private key is the lock).

## Renewal cadence

| Asset | Lifetime | Renewal action |
|-------|----------|----------------|
| Apple Distribution cert | 1 year | Same flow Steps 1-3 with a fresh CSR |
| App Store provisioning profile | 1 year | Re-download from Developer Portal (no new CSR needed) |
| Dev provisioning profile (paid) | 1 year | Auto-renewed by `make renew` |
| Dev provisioning profile (free) | 7 days | Auto-renewed by `make all` |
| Apple Developer Program | 1 year | Pay the $99 renewal |
