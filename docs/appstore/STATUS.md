# App Store launch — current status (2026-05-25)

Audit of `docs/appstore/09-checklist.md` against the actual codebase.

## DONE

### Track H — AI consent gate (Apple 5.1.2(i))

- `src/ui/consent.rs` — `ConsentScreen` component, full-screen on first launch, stores `ai_consent="true"` in SQLite settings
- `src/ui/state.rs:40` — `ai_consent: Signal<Option<bool>>` in AppState
- `src/ui/mod.rs:49` reads consent at startup, `:71` initializes signal, `:134` gates routing on `Some(true)`
- `src/services/llm.rs:64` — guard: returns `LlmError::NotConfigured` if consent absent
- `src/services/transcription/client.rs:45` — guard: returns error if consent absent
- `src/ui/settings.rs:182-183` — revocation toggle writes `"revoked"` + clears signal

### Build / signing artefacts

- `ios/entitlements.plist` exists (**TEAMID placeholder not replaced**)
- `ios/PrivacyInfo.xcprivacy` exists with `NSPrivacyCollectedDataTypeAudioData` + `NSPrivacyCollectedDataTypeOtherUserContent`, both `Linked=false`, `Tracking=false`
- `Makefile` target `appstore`: `dx build --release` → `plutil` patches → inject icon → inject PrivacyInfo → `codesign --sign "Apple Distribution"` → ditto IPA
- `Dioxus.toml` partial Info.plist: bundle_id, deployment 16.0, `background_modes = ["audio"]`, `NSMicrophoneUsageDescription`, `NSSupportsLiveActivities`, `LSRequiresIPhoneOS`, `MinimumOSVersion = 16.0`, `ITSAppUsesNonExemptEncryption = false`, `CFBundleShortVersionString = 1.0.0`, `CFBundleVersion = 1`
- Widget extension declared in `Dioxus.toml` (`recording-widget`, deployment 16.2)
- App icon: `assets/flowflow-icon-*.png` (orange gear+eyes)

### Provisioning pipeline

- `make all` auto-renew profiles
- `make renew` / `make check-profiles`
- Apple Developer Program paid + active

## Quick wins — DONE (2026-05-25)

- **Consent guard in `src/services/embed.rs`** — added `ai_consent_granted()` helper + guards in `embed_note` and `embed_attachment`. Defensive: fires before `LlmClient::from_env()` even if that path is bypassed.
- **`TEAMID` env-based substitution** — `ios/entitlements.plist` keeps `TEAMID` placeholder (committed, no secret in git). `make appstore` reads `APPLE_TEAM_ID` from `.env`, fails fast if missing, `sed`-substitutes into `/tmp/flowflow-build/entitlements.plist` for `codesign --entitlements`. Free path (`make dev/ddev/all`) untouched (Personal Team auto-provisioning).
- **`UIDeviceFamily = [1]`** — added via `plutil -replace UIDeviceFamily -json '[1]'` (iPhone only).
- **`CFBundleSupportedPlatforms = ["iPhoneOS"]`** — added via `plutil -replace CFBundleSupportedPlatforms -json '["iPhoneOS"]'`.
- **App icons hasAlpha = no** — stripped alpha from all 6 `AppIcon.xcassets/AppIcon.appiconset/icon-*.png` via `magick … -alpha remove -alpha off -strip`. Backup at `/tmp/flowflow-icons-backup/`. In-app UI icons `assets/flowflow-icon-*.png` untouched (used by `consent.rs`, `note_list.rs`, `chat/empty_state.rs`).
- **`.env.example`** — added `APPLE_TEAM_ID=` with comment explaining free vs paid path.

### i18n — 0% done

No `dioxus-i18n` dependency, no `.ftl` files, all UI strings hardcoded in French.

**Scope:**
- 35 UI files, 31 contain user-visible strings
- ~250 unique strings estimated (filtering CSS/format!/asset!/animation noise)
- Files impacted:
  - `Cargo.toml` (add `dioxus-i18n = "0.5.1"`, `sys-locale = "0.3"`, `unic-langid = "0.9"`)
  - `src/ui/mod.rs` (init `use_init_i18n` + locale detection)
  - `src/db/settings_repo.rs` (persist `ui_locale` key)
  - `src/ui/settings.rs` (language toggle)
  - `src/ui/*.rs` × 31 (replace strings with `t!()`)
  - new `assets/i18n/fr-FR.ftl`
  - new `assets/i18n/en-US.ftl`
- Effort: ~1 day solo (1h wire + 4-6h string sweep + 1h settings toggle + 1h iOS cross-compile)
- Sample current strings: "À l'instant", "Annuler", "Aucun fichier audio", "Aucun résultat", "Aucun thème", "Aucune conversation", "Auto-tag", "Bienvenue", "Cette action est irréversible.", "Chat avec tes notes", "Chercher mes notes..."

### Privacy policy

- Draft ready: `docs/appstore/07-privacy-policy-draft.md` (EN + FR)
- Not published to a public URL
- Action: push to a GitHub Pages site under `mirkobozzetto.github.io/flowflow-privacy` or similar
- Required to fill App Store Connect "Privacy Policy URL" field

### Assets

- ~~App icon 1024×1024 PNG sans alpha~~ — **DONE** (2026-05-25): all `AppIcon.xcassets/AppIcon.appiconset/icon-*.png` (76, 120, 152, 167, 180, 1024) stripped of alpha. Backup: `/tmp/flowflow-icons-backup/`.
- Screenshots: **0 generated**. Need 7 slots @ 1320×2868 (iPhone 6.9") in FR + EN
  - Use iPhone 16/17 Pro Max simulator
  - `xcrun simctl io booted screenshot screenshot.png`
  - Workflow tool: Screenshot Otter or ScreenMaker
- Optional preview video: skip for V1

### App Store Connect

- App record not created
- Metadata not entered (name, subtitle, description, keywords, category, age rating)
- Privacy nutrition labels not filled (Audio Data + Other User Content, both Not Linked, no Tracking)
- Pricing not set (Free)

### Privacy / GDPR

- DPA Soniox: **not requested** (email support@soniox.com)
- DPA OpenAI: auto-included in API Business Terms (verify account enrolled)
- DPA Anthropic: auto-included in Commercial Terms (verify account enrolled)
- Optional: switch Soniox to EU endpoint `api.eu.soniox.com` (Settings toggle)

### Apple Developer Portal

- Apple Distribution cert: status unknown (user must verify in Xcode → Settings → Accounts → Manage Certificates)
- iPhone test device enrolled in Devices list: confirmed (used by `make ddev`)
- Team ID: known (used by provisioning renewal scripts, get from `security find-identity -v -p codesigning`)

## TECHNICAL STEPS REMAINING

### Screenshots / app demo

1. Boot iPhone 17 Pro Max simulator: `xcrun simctl boot "iPhone 17 Pro Max"`
2. Seed app with realistic demo content (5-10 sample notes, 1-2 folders, 1 chat thread)
3. Capture 7 slots per story (see `04-screenshots.md` plan):
   - 1: Recording bar + waveform
   - 2: Transcribed note
   - 3: RAG chat + sources
   - 4: NotesList + tag chips
   - 5: Folders / sidebar
   - 6: Attachment PDF/DOCX import
   - 7: Settings provider picker
4. `xcrun simctl io booted screenshot slot-N.png`
5. Drop in Screenshot Otter → caption FR + EN per slot
6. Export ZIP per language → upload via App Store Connect Media Manager

### App preview video (optional V1)

- Skip per `04-screenshots.md` recommendation
- If needed later: 15-30s, .mov H.264 + AAC, max 500MB 30fps, real in-app footage

### Build pipeline finalization

- Wire missing plutil keys (`UIDeviceFamily`, `CFBundleSupportedPlatforms`) into `make appstore`
- Replace `TEAMID` in `ios/entitlements.plist` with real ID
- Test `make appstore` end-to-end: produce `FlowFlow.ipa` at repo root
- Validate IPA: `xcrun altool --validate-app -f FlowFlow.ipa -t ios -u APPLE_ID -p APP_SPECIFIC_PASSWORD`
- Upload via Transporter.app (drag IPA) or `xcrun altool --upload-app`

## ONGOING COMPLIANCE (post-launch evolution)

### When adding a new AI provider or data destination

- Update `ConsentScreen` text to name the new provider explicitly (5.1.2(i) compliance)
- Re-trigger consent: bump `ai_consent` schema (store `{version, providers, timestamp}` JSON instead of just `"true"`); on version mismatch → show consent again
- Update privacy policy live URL + App Store Connect privacy labels
- Update `PrivacyInfo.xcprivacy` if new data type collected
- Update DPA inventory

### When adding new permissions or APIs

- Each new `NSXxxUsageDescription` string in `Dioxus.toml [ios.plist]` must be specific (Apple rejects vague strings)
- Each new Required Reason API used (file timestamps via `std::fs::metadata`, UserDefaults, etc.) must be declared in `PrivacyInfo.xcprivacy` `NSPrivacyAccessedAPITypes` with a valid reason code
- Crate audit: every new Cargo dep must be checked for IDFA / AdSupport / tracking SDKs (use `cargo tree | grep -i 'ad\|track\|analytics'`)

### When adding a new language

- Add `assets/i18n/<locale>.ftl`
- Register in `use_init_i18n` config
- Add language option in Settings toggle
- Create App Store Connect localization (name, subtitle, description, keywords, screenshots) — fallback to FR/EN if missing

### Apple guideline drift

- Subscribe to Apple Developer News (https://developer.apple.com/news/) for guideline changes
- Re-audit consent screen + privacy manifest every major iOS release (Sep each year)
- Re-validate `PrivacyInfo.xcprivacy` Required Reason API list — Apple expands the list periodically

### Build artefacts hygiene

- `CFBundleVersion`: increment on every TestFlight upload (Apple rejects duplicate version + build number combos)
- `CFBundleShortVersionString`: bump only on user-visible release (1.0.0 → 1.0.1 → 1.1.0)
- Keep `entitlements.plist` Team ID synced if Apple ever rotates it (rare)
- Provisioning profiles: paid Developer Program = 1 year, `make check-profiles` keeps inventory; auto-renew via `make renew`

## PRIORITY ORDER (shortest path to first submit)

1. ~~Replace TEAMID~~ → **DONE** (env-based substitution via `APPLE_TEAM_ID`)
2. ~~Add `UIDeviceFamily` + `CFBundleSupportedPlatforms`~~ → **DONE**
3. ~~Add consent guard in `embed.rs`~~ → **DONE**
4. ~~Verify icon `hasAlpha = no`~~ → **DONE** (alpha stripped on AppIcon.xcassets)
5. **Publish privacy policy on GitHub Pages** (1-2h)
6. **i18n EN: dioxus-i18n wire + extract strings + en-US.ftl** (1 day)
7. **Apple Distribution cert verify in Xcode** (15 min)
8. **DPA Soniox email request** (5 min)
9. **`make appstore` end-to-end test** (1h debugging) — requires `APPLE_TEAM_ID` in `.env`
10. **Screenshots × 7 slots × 2 langues** (3-4h)
11. **App Store Connect record + metadata + nutrition labels** (2-3h)
12. **Upload IPA via Transporter** (30 min)
13. **App Review Notes + API keys reviewer + IDFA=no + export compliance** (30 min)
14. **Submit for Review**

Total realistic: 2-3 dev days + 1 admin day.

Track H is shipped — the previously blocking item — so the path is mostly mechanical now.
