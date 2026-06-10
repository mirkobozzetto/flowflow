# iOS provisioning and App Store distribution

> Moved out of the README to keep it slim. Everything operational about signing, provisioning, and shipping to the App Store lives here.

## iOS Device Provisioning

Running on a physical iPhone requires a valid provisioning profile. Apple enforces code signing for all device builds.

**Free Apple account**: profiles expire after 7 days. **Paid Apple Developer Program** (€99/year): profiles last 1 year, plus App Store and TestFlight access.

`make all` auto-detects expiry < 24h and runs `make renew` before building. Renewal uses a minimal Xcode project template under `tools/provision-renew/` invoked via `xcodebuild -allowProvisioningUpdates`. Zero manual Xcode GUI required.

```bash
make check-profiles  # show expiration dates
make renew           # regenerate now
```

**On your iPhone** (first time only): Settings → General → VPN & Device Management → tap your developer certificate → Trust.

Once your paid Apple Developer Program is active, profiles last 1 year and `make renew` becomes a once-a-year operation.

## App Store distribution

`make appstore` produces a `FlowFlow.ipa` that passes Apple's validator with zero errors. The Dioxus CLI omits several keys required for App Store distribution (`CFBundlePackageType`, `DT*`, widget `UIRequiredDeviceCapabilities`, ...) and uses a `ditto` invocation that strips the `Payload/` directory; the Makefile patches all of these. Full breakdown of every Apple validation error we hit and the corresponding fix: [../deploy/04-dioxus-app-store-workarounds.md](../deploy/04-dioxus-app-store-workarounds.md). Upstream tracking: [Dioxus issue #3817](https://github.com/DioxusLabs/dioxus/issues/3817).

End-to-end submission walkthrough: [../deploy/02-app-store-submission.md](../deploy/02-app-store-submission.md). Sequential execution plan: [../deploy/03-execution-plan.md](../deploy/03-execution-plan.md).

### Optional: server-side IPA validation

`make appstore` can call `xcrun altool --validate-app` against Apple's servers after the IPA is built, so you catch rejections (missing keys, wrong cert, version conflict) before opening Transporter. Add these to `.env`:

```
APPLE_ID=you@example.com
APP_SPEC_PASSWORD=xxxx-xxxx-xxxx-xxxx
```

Generate the app-specific password at https://account.apple.com/account/manage/section/security → **App-Specific Passwords** → **Generate Password**. Treat it like a secret. If either env var is missing, the validation step is skipped silently and you can still drag the IPA into Transporter manually.

### Troubleshooting `make appstore` after a fresh Xcode install

A fresh Xcode install (e.g. upgrading 26.4.1 → 26.5 via the `.xip` from developer.apple.com) wipes the local Apple ID account, the provisioning profiles folder, and the SDK stat caches. `make appstore` will then fail with one of the errors below. The fix is sequential, do them in this order:

1. **`error: No Accounts: Add a new account in Accounts settings.`**
   Open Xcode → `cmd+,` → **Accounts** → click **+** → **Apple ID** → log in with the developer account. Your paid team (`R477R8NK27` in our case) appears automatically because the certs are already in the Keychain.

2. **`No provisioning profiles found when trying to codesign the app.`**
   Run `make renew`. This regenerates the iOS App Development profiles in `~/Library/Developer/Xcode/UserData/Provisioning Profiles/`. Verify with `make check-profiles`.

3. **`ERROR: no App Store provisioning profile for com.mirkobozzetto.flowflow.`**
   `make renew` only creates *development* profiles. The App Store *distribution* profiles must be downloaded manually from Apple Developer Portal:
   - Go to https://developer.apple.com/account/resources/profiles/list
   - Click **FlowFlow App Store** → **Download** → double-click the `.mobileprovision` to install it
   - Repeat for **flowflow recording-widget**
   - Verify: `ls ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/` should show 4 files (2 dev + 2 App Store)

4. **`ClangStatCache failed with a nonzero exit code`**
   Xcode 26.5 fresh install sometimes corrupts its SDK stat cache. Wipe it:
   ```bash
   rm -rf ~/Library/Developer/Xcode/DerivedData
   rm -rf /var/folders/*/C/com.apple.DeveloperTools/26.5-*/Xcode/SDKStatCaches.noindex/
   sudo chown -R $USER ~/Library/Developer/Xcode/DerivedData 2>/dev/null
   ```
   Xcode will rebuild the cache on the next invocation.

5. **`Ce build utilise une version bêta de Xcode et ne peut donc pas être soumis.`** (ASC submission error)
   The currently-active Xcode is older than the latest stable release. Update to the newest stable Xcode (App Store first, fallback to `.xip` download at https://developer.apple.com/download/all/?q=xcode), then `sudo xcode-select -s /Applications/Xcode.app/Contents/Developer` and rebuild.

### Glossary

- **IPA** - iOS App Archive. A signed `.ipa` is the zipped, signed bundle Apple accepts. `make appstore` builds it.
- **ASC** - [App Store Connect](https://appstoreconnect.apple.com). Apple's web dashboard to upload builds, fill metadata, attach screenshots, and submit for review.
- **Transporter** - [free Mac app](https://apps.apple.com/app/transporter/id1450874784) to upload an `.ipa` to ASC.
- **Provisioning profile** - Apple signed file that authorizes your bundle ID + cert combo. Generated once at [developer.apple.com](https://developer.apple.com/account/resources/profiles/list).
- **CFBundleVersion / CFBundleShortVersionString** - build number (must increment every upload) and user-visible version (e.g. `1.0.0`).
- **Bundle ID** - unique app identifier (`com.mirkobozzetto.flowflow`). Tied to the provisioning profile.

### Manual steps before submitting for review

Everything below has to be done by hand in the App Store Connect web UI or on the simulator. Tooling cannot automate these.

1. **Generate 2 review-only API keys, capped at $5 each.** Create dedicated keys (not your prod keys) for the Apple reviewer, with hard spending limits. Revoke after approval.
   - OpenAI key: https://platform.openai.com/api-keys → spending cap at https://platform.openai.com/settings/organization/limits
   - Soniox key: https://console.soniox.com/api-keys → top up $5 prepaid credit

2. **Capture screenshots on the iOS simulator** (1284 × 2778 for iPhone 6.5" slot, or 1320 × 2868 for 6.9").
   ```bash
   xcrun simctl boot "iPhone 17 Pro Max"
   xcrun simctl io booted screenshot slot-1.png
   ```
   Take 3 to 4 screens showing the app in English. Upload them in the iPhone slot on the ASC version page (see Quick links below).

3. **Fill App Review Information** (bottom of the ASC version page):
   - **Contact**: your first name, last name, email, phone number.
   - **Sign-in required**: check the box, then paste the 2 review API keys + setup instructions in the Notes field. Template:
     ```
     FlowFlow requires 2 API keys for AI features:
     - Soniox API Key: <paste review key>
     - OpenAI API Key: <paste review key>

     Setup: launch app → Settings (gear icon) → paste both keys.
     AI consent screen appears on first launch, tap "Enable".
     Microphone permission requested on first recording.
     Keys will be revoked after approval. Please limit testing to ~5-10 transcriptions/queries.
     ```

4. **Click "Add for Review"** (top right of the version page) once every section shows a green check, then **"Submit for Review"** on the next screen. Apple usually responds within 24 to 48 hours.

### Quick links (App Store dashboards)

| Where | URL |
|-------|-----|
| App Store Connect - FlowFlow distribution | https://appstoreconnect.apple.com/apps/6773033233/distribution |
| ASC - App Information (name, subtitle, primary language, category) | https://appstoreconnect.apple.com/apps/6773033233/distribution/info |
| ASC - Version page (description, promo text, keywords, support URL, copyright) | https://appstoreconnect.apple.com/apps/6773033233/distribution/ios/version/inflight |
| ASC - Privacy (policy URL + nutrition labels) | https://appstoreconnect.apple.com/apps/6773033233/distribution/privacy |
| ASC - Age rating questionnaire | https://appstoreconnect.apple.com/apps/6773033233/distribution/ratings/ios |
| ASC - Pricing and availability | https://appstoreconnect.apple.com/apps/6773033233/distribution/pricing |
| ASC - Submit for review | https://appstoreconnect.apple.com/apps/6773033233/distribution/reviewsubmissions |
| Apple Developer Portal (certs + profiles) | https://developer.apple.com/account |
| OpenAI API keys + spending limits | https://platform.openai.com/api-keys |
| Soniox console (API keys, billing) | https://console.soniox.com |
| Privacy policy (live, GitHub Pages) | https://mirkobozzetto.github.io/flowflow-privacy/ |
| Privacy policy source repo | https://github.com/mirkobozzetto/flowflow-privacy |

### Push an update (continuous releases)

Workflow once the app is live on the Store:

1. Bump `CFBundleVersion` in `Makefile` (line 74) by `+1`. ASC rejects duplicate build numbers. Bump `CFBundleShortVersionString` only on user-visible releases (e.g. `1.0.0` → `1.0.1`).
2. `make appstore` → fresh signed `FlowFlow.ipa`.
3. Open Transporter → drag the `.ipa` → Deliver. Apple processes the build (5-30 min).
4. ASC → your app → **TestFlight or App Store tab** → attach the new build to the current version, or create a new version (`+ Version` button) if you bumped the short version string.
5. Update locale-specific metadata (description, keywords, screenshots) if needed - see *Multilingual releases* below.
6. **Submit for Review**. Apple usually answers within 24h.

### Multilingual releases

The app auto-detects the iPhone system language at boot (NSLocale) and falls back to English. Users can switch language any time via Settings → Langue / Language. Supported locales: `en`, `fr`.

For ASC: add **English (U.S.)** and **French (France)** under App Information → Localizations. Each locale needs its own screenshots (iPhone 6.9", 1320×2868), description, and keywords. Upload locale screenshots separately in each language tab.
