# App Store submission — detailed steps

This guide picks up FlowFlow after `make appstore` (IPA produced) and runs through to "Submit for Review". Follow the exact order to avoid ping-ponging between App Store Connect and the terminal.

Prerequisite: `FlowFlow.ipa` exists at the repo root. Check:

```bash
ls -lh FlowFlow.ipa
```

Must show ~60 MB.

## Overview

Six steps, in order:

1. Install Transporter (10 min, one time)
2. Upload `FlowFlow.ipa` (5 min upload + 30 min Apple processing)
3. Publish the privacy policy on GitHub Pages (15 min)
4. Generate iPhone 6.9" screenshots (30-60 min)
5. Fill App Store Connect metadata (30-60 min)
6. Submit for Review (1 min)

Total: ~3-4h of focused work, plus 24-72h Apple review wait.

## 1. Install Transporter

Transporter is Apple's official app to upload IPAs to App Store Connect. Free on the Mac App Store.

### Option A — install via Mac App Store GUI

1. Open the `App Store` app on your Mac.
2. Search for `Transporter`.
3. Click `Get` then `Install`.

### Option B — install via `mas` CLI

`mas` is a third-party Mac App Store CLI. Not installed on your machine yet:

```bash
brew install mas
mas install 1450874784
```

### Verify

```bash
ls /Applications/ | grep -i transporter
```

Must return `Transporter.app`.

## 2. Upload FlowFlow.ipa

### Launch Transporter

```bash
open -a Transporter
```

On first launch, sign in with your Apple Developer ID (`mirko@mirko.re`). Regular password, not app-specific.

If Apple asks for 2FA (6-digit code on iPhone): approve via Settings → Apple ID → Apple ID Verification Code on your iPhone.

### Drop the IPA

1. Transporter window open: drag-drop `FlowFlow.ipa` from Finder into the window.
2. Transporter validates the IPA (signature, entitlements, Info.plist) in 30s to 2min. On error → see Troubleshooting below.
3. If OK: `Deliver` button appears → click.
4. Upload takes 1-5 min depending on your connection.
5. End state: status `Delivered`.

### Apple processes the build

After `Delivered`: Apple processes the build server-side (~15-30 min). Meanwhile:

- Email from Apple: "Your build is processing".
- When ready: email "Your build is ready for testing on TestFlight".
- Build appears in App Store Connect → your app → tab `TestFlight`.

You can continue steps 3-5 during processing.

## 3. Publish the privacy policy

App Store Connect requires a **public URL** for your privacy policy. You already have an FR + EN draft in `docs/appstore/07-privacy-policy-draft.md`.

Simple solution: GitHub Pages on a dedicated repo.

### Create the flowflow-privacy repo

```bash
cd /tmp
mkdir flowflow-privacy && cd flowflow-privacy
cp /Users/mirkobozzetto/code/flowflow/docs/appstore/07-privacy-policy-draft.md index.md
git init
git add index.md
git commit -m "Initial privacy policy"
```

Create the remote repo via GitHub CLI:

```bash
gh repo create mirkobozzetto/flowflow-privacy --public --source=. --push
```

### Enable GitHub Pages

```bash
gh api -X POST /repos/mirkobozzetto/flowflow-privacy/pages \
  -f source[branch]=main -f source[path]=/
```

Or via web: https://github.com/mirkobozzetto/flowflow-privacy/settings/pages → Source: Deploy from a branch → Branch: `main` / root → Save.

Wait 1-2 min, your URL becomes:

```
https://mirkobozzetto.github.io/flowflow-privacy/
```

Verify in the browser that the page loads. **Save this URL**, you need it at step 5.

## 4. Generate screenshots

Apple requires 3 to 10 iPhone 6.9" screenshots (1320×2868 px) for submission. FR + EN if you target both languages.

### Prepare the simulator

```bash
xcrun simctl boot "iPhone 17 Pro Max"
open -a Simulator
```

Check the device resolution:

```bash
xcrun simctl list devices | grep "iPhone 17 Pro Max"
```

The iPhone 17 Pro Max boots at 1320×2868. If you don't have this simulator:

```bash
xcrun simctl list runtimes
xcrun simctl create "iPhone 17 Pro Max" "iPhone 17 Pro Max" iOS18.5
```

### Launch FlowFlow in the simulator

```bash
cd /Users/mirkobozzetto/code/flowflow
make dev
```

App launches. On first launch: ConsentScreen → tap `C'est parti !`.

### Seed demo content

For credible screenshots, seed the app with:

- 5-10 transcribed voice notes (varied topics: meeting, idea, reminder, todo, quote)
- 1-2 folders (`Work`, `Personal`)
- 1 chat conversation with sources
- 1 imported PDF attachment

5 min of manual setup. Do NOT put sensitive data (anything shown in screenshots will be public).

### Capture the 7 slots

Strategy: 1 screenshot = 1 key feature. Narrative order:

1. **Recording bar + waveform** — live voice recording view
2. **Transcribed note** — NoteDetail with title + transcription + tags
3. **RAG chat + sources** — ChatView with question + answer + cited sources
4. **NotesList** — note list with colored tag chips
5. **Folders sidebar** — sidebar open with folder tree
6. **PDF attachment** — NoteDetail with attachment card + modal viewer
7. **Settings** — provider picker OpenAI/Anthropic + API keys

For each slot, in the simulator, navigate to the target view, then:

```bash
xcrun simctl io booted screenshot ~/Desktop/slot-1.png
```

Repeat for slot-2.png to slot-7.png.

### Verify resolution

```bash
sips -g pixelWidth -g pixelHeight ~/Desktop/slot-1.png
```

Must show `1320 × 2868`. Otherwise the simulator is misconfigured.

### FR + EN captions (optional but recommended)

App Store accepts raw screenshots or with captions (explanatory text overlay).

Simple tool: **Screenshot Otter** (Mac App Store, paid ~10€) or **ScreenMaker** (free). Lets you add captions on top.

Without a tool: upload raw screenshots. OK for V1.

### Expected output

```
~/Desktop/
  slot-1.png  (1320×2868)
  slot-2.png  (1320×2868)
  ...
  slot-7.png  (1320×2868)
```

You can do the same for EN later (relaunch the app in EN mode once i18n is done).

## 5. Fill App Store Connect metadata

Back to https://appstoreconnect.apple.com/apps → click your `FlowFlow` app → sidebar `iOS App Version 1.0`.

### 5a. iPhone screenshots

On the Version 1.0 page (where you are now), section `Previews and Screenshots`:

1. Tab `iPhone` selected.
2. Drag-drop your 7 PNGs from `~/Desktop/` into the `Drag up to 10 screenshots here` area.
3. Apple reorders by drag. Slot 1 = first impression (recording bar).

Apple does not require all 10. **3 is enough** to submit, but 7 = better. Slots 1 and 2 are the most-viewed in App Store search results.

### 5b. Promotional text + description + keywords

Scroll down on the same page:

| Field | Limit | Suggested value |
|-------|-------|-----------------|
| `Promotional text` | 170 chars | `Voice notes transcribed by AI, organized with tags and folders, with RAG chat over your notes. 100% local on your iPhone.` |
| `Description` | 4000 chars | see block below |
| `Keywords` | 100 chars | `voice,notes,AI,chat,RAG,transcription,audio,Soniox,GPT,Claude` |
| `Support URL` | URL | `https://github.com/mirkobozzetto/flowflow` |
| `Marketing URL` | URL (optional) | empty for V1 |

#### Full description (copy-paste)

```
FlowFlow turns your thoughts into structured notes in seconds.

RECORD
Tap the mic icon. Speak. FlowFlow captures your voice with native iOS audio quality, then Soniox transcribes it in seconds.

ORGANIZE
AI-generated tags. Hierarchical folders. Semantic search over your notes via local OpenAI embeddings. Everything lives on your iPhone.

CHAT WITH YOUR NOTES
Ask a question. FlowFlow finds relevant passages and answers with cited sources. Pick your provider: OpenAI (GPT) or Anthropic (Claude).

IMPORT DOCUMENTS
Add PDF, DOCX, MD or CSV files to a note. FlowFlow indexes them and makes them searchable alongside the rest.

LOCAL-FIRST
Your data stays on your iPhone. Local SQLite + LanceDB. The only external calls are to APIs you explicitly configure (transcription, AI).

100% RUST
Built with Dioxus, cpal, rig-core, LanceDB. Native performance, optimized memory, zero JavaScript.

FREE, OPEN SOURCE
Source code available. Bring your own API keys (OpenAI, Anthropic, Soniox).
```

### 5c. Left sidebar — other pages

Left sidebar → click `App Information`:

- **Subtitle** (30 chars): `Voice notes + AI chat`
- **Primary Category**: `Productivity`
- **Secondary Category**: `Utilities`
- **Content Rights**: `No, my app does not contain, show, or access third-party content`
- **Age Rating**: click `Edit` → questionnaire (answer `None` everywhere except Apps with User-Generated Content → `Infrequent/Mild` since FlowFlow is local-only, solo) → Done

Sidebar → `Pricing and Availability`:

- **Price**: `Free (CHF 0.00 / EUR 0.00)`
- **Availability**: all countries by default, or restrict to EU/Switzerland if you prefer a staged rollout

Sidebar → `App Privacy`:

- **Privacy Policy URL**: paste the GitHub Pages URL from step 3
- **Data Practices**: click `Get Started`
  - **Audio Data**: Yes collected
    - Linked to user: No (FlowFlow does not link audio to an account)
    - Tracking: No
    - Purpose: `App Functionality` (transcription)
  - **User Content (notes)**: Yes collected
    - Linked to user: No
    - Tracking: No
    - Purpose: `App Functionality`
  - **Identifiers**: No
  - **Usage Data**: No
  - **Diagnostics**: No
  - **Contact Info**: No
  - **Financial Info**: No
  - **Health & Fitness**: No
  - **Purchases**: No
  - **Search History**: No

Save each section.

### 5d. App Review

Sidebar → `App Review`:

- **Sign-In Required**: `No` (FlowFlow has no login)
- **Notes**: text for the Apple reviewer:

```
FlowFlow is a voice-note app with AI transcription and RAG chat.

To test:
1. Launch the app — a consent screen appears explaining AI data flows. Tap "C'est parti !" to accept.
2. Tap the floating + button to create a note.
3. Tap the microphone icon to record a voice note. Speak for 5-10 seconds, then stop.
4. The transcription takes ~3 seconds (Soniox API).
5. Open the Settings tab to configure your own API keys, or use the test keys below.
6. From the home screen, tap the chat icon to ask a question about your notes.

Test API keys are pre-configured in the build. No additional sign-in needed.

The app does not collect personal data and does not track users. All notes are stored locally on the device (SQLite + LanceDB).

System requirements: iOS 16.0+. iPhone only.
```

- **Contact Info**: name (Mirko Bozzetto), email (`mirko@mirko.re`), phone (+32 484 906 499)
- **Demo Account**: leave empty (no login in FlowFlow)
- **Additional Notes**: leave empty

### 5e. TestFlight Build

Sidebar → back to `iOS App Version 1.0`. Scroll to `Build`.

If Apple finished processing your IPA (step 2), it appears here. Click `Select a build` → choose `1.0 (1)`.

If nothing appears: wait. Processing can take up to 1h in extreme cases.

### 5f. Export Compliance

`Export Compliance` section (under Build):

- Question: `Does your app use only standard iOS encryption algorithms?` → **Yes**
  - Justification: FlowFlow uses HTTPS via `reqwest` to call external APIs (OpenAI, Anthropic, Soniox), but does not implement custom encryption.

### 5g. IDFA (Advertising)

`Advertising Identifier (IDFA)` section:

- `Does this app use the Advertising Identifier (IDFA)?` → **No**

## 6. Submit for Review

Once all sections above are complete (left sidebar: green ✓ everywhere), click the `Add for Review` button top-right.

Apple confirms and moves status to `Waiting for Review` (24-72h typical), then `In Review`, then `Ready for Distribution` (if OK) or `Rejected` (with reasons).

If rejected:

- Read Apple's feedback carefully (sent by email + visible in `App Review`)
- Fix code or metadata
- Re-submit: `Add for Review` becomes clickable again

## Transporter troubleshooting

### `ITMS-90717: Invalid App Store Icon`

The 1024×1024 icon has an alpha channel. You already stripped it (`make appstore` produces a correct IPA), but if you rebuild from scratch: check `AppIcon.xcassets/AppIcon.appiconset/icon-1024.png` and verify `sips -g hasAlpha icon-1024.png` → `no`.

### `ITMS-90209: Invalid Segment Alignment`

Binary was not properly signed for distribution. Likely wrong cert (Apple Development instead of Apple Distribution). Re-run `make appstore` after verifying:

```bash
security find-identity -v -p codesigning | grep "Apple Distribution"
```

### `Missing Push Notification Entitlement`

You enabled Push Notifications in Apple Developer Portal but the entitlement is not in `ios/entitlements.plist`. FlowFlow does not use Push for V1 → disable the capability on the App ID in Apple Developer Portal.

### `Invalid Provisioning Profile`

The `.mobileprovision` embedded in the IPA is not an App Store profile. Verify:

```bash
unzip -p FlowFlow.ipa "Payload/Flowflow.app/embedded.mobileprovision" \
  | security cms -D -i - | grep -A1 ProvisionsAllDevices
```

If you see `<true/>` or the `ProvisionedDevices` key → it's a dev profile, not App Store. Redo step 5 of the `01-fresh-setup-from-scratch.md` guide.

## Minimal recommended backup

Before clicking Submit, copy to a safe place:

- `secrets/ios/distribution.key` (1Password or encrypted disk)
- `secrets/ios/distribution.cer`
- Both `.mobileprovision` files
- Your Team ID: `R477R8NK27`
- Your bundle IDs: `com.mirkobozzetto.flowflow` + `.recording-widget`

If you lose the `.key`, you can never sign a FlowFlow update again without revoking the cert and restarting from scratch.

## Timing summary

| Step | Action | Duration |
|------|--------|----------|
| 1 | Install Transporter | 10 min |
| 2 | Upload IPA | 5 min + 30 min processing |
| 3 | GitHub Pages privacy policy | 15 min |
| 4 | Screenshots × 7 | 30-60 min |
| 5 | ASC metadata | 30-60 min |
| 6 | Submit | 1 min |
| Apple | Review | 24-72h |
| Active total | | ~3-4h |
