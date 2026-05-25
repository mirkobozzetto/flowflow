# App Store launch — sequential execution plan

Condensed plan to go from produced IPA to "Submitted for Review", in exact order.

Validated prerequisites:
- `FlowFlow.ipa` at the repo root (~60 MB)
- `secrets/ios/` backed up to Google Drive (private)
- PR #12 merged on `main`

Total: ~3-4h active + 24-72h Apple review.

---

## Step 1 — Privacy policy published (15-20 min)

Apple requires a public URL for the policy. Solution: dedicated GitHub repo + GitHub Pages.

### 1.1 Create the `flowflow-privacy` repo

```bash
mkdir -p /tmp/flowflow-privacy && cd /tmp/flowflow-privacy
cp /Users/mirkobozzetto/code/flowflow/docs/appstore/07-privacy-policy-draft.md index.md
git init -b main
git add index.md
git commit -m "Initial privacy policy"
gh repo create mirkobozzetto/flowflow-privacy --public --source=. --push
```

### 1.2 Enable GitHub Pages

```bash
gh api -X POST /repos/mirkobozzetto/flowflow-privacy/pages \
  -f 'source[branch]=main' -f 'source[path]=/'
```

Or web UI: https://github.com/mirkobozzetto/flowflow-privacy/settings/pages → Source `Deploy from a branch` → `main` / root → Save.

### 1.3 Verify the URL

Wait 1-2 min, then:

```bash
curl -sI https://mirkobozzetto.github.io/flowflow-privacy/ | head -5
```

Must return `HTTP/2 200`. **Save the exact URL** — required at step 4.

---

## Step 2 — Soniox DPA requested (5 min)

Soniox does not publish its DPA online. Request by email to have paperwork in case of a GDPR audit.

### 2.1 Send the email

To: `support@soniox.com`
Subject: `DPA request — FlowFlow (iOS app, Belgium)`

Body (copy-paste):

```
Hello,

I am the developer of FlowFlow, an open-source iOS app that uses the Soniox
async transcription API for voice notes. The app is operated as a free
indie product from Brussels, Belgium (EU/GDPR jurisdiction).

Could you please share:

1. Your Data Processing Agreement (DPA) for EU developers.
2. Confirmation that customer audio is not used to train Soniox models.
3. Default retention period for async-uploaded audio, and how to trigger
   deletion (programmatic API or namespace cleanup).
4. Whether the EU endpoint (api.eu.soniox.com) is recommended for
   EU-based end users to keep data within the EU.

For context: each FlowFlow user supplies their own API key, the app holds
no server, and audio is uploaded directly from device to Soniox.

Thanks,
Mirko Bozzetto
mirko@mirko.re
https://github.com/mirkobozzetto/flowflow
```

### 2.2 Archive the response

When Soniox replies → save the email + DPA PDF in Google Drive next to `secrets/ios/`. Not blocking for the Apple submission, but needed in case of audit.

---

## Step 3 — Transporter installed (10 min)

Official Apple app to upload IPAs. No reliable CLI alternative.

### 3.1 Install via Mac App Store

```bash
open "macappstore://apps.apple.com/app/transporter/id1450874784"
```

Click `Get` → `Install`. Wait for download to finish.

### 3.2 Verify

```bash
ls /Applications/ | grep -i transporter
```

Must show `Transporter.app`.

### 3.3 Sign in

```bash
open -a Transporter
```

Login with Apple Developer ID (`mirko@mirko.re`), regular password. 2FA via iPhone if requested.

---

## Step 4 — App Store Connect: create the app (15 min)

### 4.1 Go to ASC

URL: https://appstoreconnect.apple.com/apps → `+` button → `New App`.

### 4.2 Creation form

| Field | Value |
|-------|-------|
| Platforms | iOS |
| Name | `FlowFlow` |
| Primary Language | `English (U.S.)` |
| Bundle ID | `com.mirkobozzetto.flowflow` (select in dropdown, the App ID you created) |
| SKU | `flowflow-001` (internal, free choice) |
| User Access | `Full Access` |

`Create`. The app appears in the list with status `Prepare for Submission`.

---

## Step 5 — Upload the IPA (5 min + 30 min processing)

### 5.1 Drop into Transporter

1. Transporter window open → drag `/Users/mirkobozzetto/code/flowflow/FlowFlow.ipa` into it.
2. Transporter validates (signature, entitlements, Info.plist) → 30s-2min.
3. If OK: `Deliver` button → click.
4. Upload: 1-5 min depending on connection.
5. Final status: `Delivered`.

### 5.2 Wait for Apple processing

Email Apple "Your build is processing" → 15-30 min.
Email Apple "Your build is ready for testing on TestFlight" → build available in ASC tab `TestFlight`.

Continue steps 6-7 during processing.

### 5.3 Troubleshooting

- `ITMS-90717 Invalid App Store Icon`: alpha not stripped → `sips -g hasAlpha AppIcon.xcassets/AppIcon.appiconset/icon-1024.png` must say `no`. Already done, should be OK.
- `Invalid Provisioning Profile`: dev profile instead of App Store. Check with:
  ```bash
  unzip -p FlowFlow.ipa "Payload/Flowflow.app/embedded.mobileprovision" \
    | security cms -D -i - | grep -E "ProvisionsAllDevices|ProvisionedDevices" || echo "OK App Store profile"
  ```
- `ITMS-90209 Invalid Segment Alignment`: wrong cert. Check `security find-identity -v -p codesigning | grep "Apple Distribution"`.

---

## Step 6 — Screenshots × 7 (45-60 min)

### 6.1 Boot iPhone 17 Pro Max simulator

```bash
xcrun simctl list devices | grep "iPhone 17 Pro Max"
xcrun simctl boot "iPhone 17 Pro Max" 2>/dev/null || true
open -a Simulator
```

Expected resolution: **1320 × 2868**. If the device is missing:

```bash
xcrun simctl list runtimes
xcrun simctl create "iPhone 17 Pro Max" "iPhone 17 Pro Max" iOS18.5
```

### 6.2 Launch FlowFlow

```bash
cd /Users/mirkobozzetto/code/flowflow
make dev
```

ConsentScreen → `C'est parti !`.

### 6.3 Seed demo content

Manually inside the app, create:

- 5 transcribed voice notes (varied topics: client meeting, product idea, todo, quote, reminder)
- 2 folders: `Work`, `Personal`
- 1 chat conversation ("Summarize my notes on project X")
- 1 PDF attachment imported into a note

No sensitive data — everything will be public.

### 6.4 Capture the 7 slots

For each view, navigate → capture:

```bash
xcrun simctl io booted screenshot ~/Desktop/slot-1.png  # RecordingBar live
xcrun simctl io booted screenshot ~/Desktop/slot-2.png  # NoteDetail transcribed + tags
xcrun simctl io booted screenshot ~/Desktop/slot-3.png  # ChatView + sources
xcrun simctl io booted screenshot ~/Desktop/slot-4.png  # NotesList + tag chips
xcrun simctl io booted screenshot ~/Desktop/slot-5.png  # Sidebar folders
xcrun simctl io booted screenshot ~/Desktop/slot-6.png  # NoteDetail + AttachmentModal PDF
xcrun simctl io booted screenshot ~/Desktop/slot-7.png  # Settings provider picker
```

### 6.5 Verify resolution

```bash
for f in ~/Desktop/slot-*.png; do sips -g pixelWidth -g pixelHeight "$f" | tail -2; done
```

Must show `1320 / 2868` for all.

---

## Step 7 — ASC metadata (45-60 min)

Back to https://appstoreconnect.apple.com/apps → click `FlowFlow` → sidebar `iOS App Version 1.0`.

### 7.1 iPhone screenshots

`Previews and Screenshots` section → tab `iPhone 6.9"` → drag-drop `~/Desktop/slot-1.png` to `slot-7.png` in order.

Apple requires only 3 minimum. 7 = full coverage.

### 7.2 Promo text + description + keywords

| Field | Limit | Value |
|-------|-------|-------|
| Promotional Text | 170 chars | `Voice notes transcribed by AI, organized with tags and folders, with RAG chat over your notes. 100% local on your iPhone.` |
| Keywords | 100 chars | `voice,notes,AI,chat,RAG,transcription,audio,Soniox,GPT,Claude` |
| Support URL | URL | `https://github.com/mirkobozzetto/flowflow` |
| Marketing URL | URL | (empty V1) |

**Description** (copy-paste, already written in `docs/deploy/02-app-store-submission.md` section 5b).

### 7.3 App Information (left sidebar)

- Subtitle (30 chars): `Voice notes + AI chat`
- Primary Category: `Productivity`
- Secondary Category: `Utilities`
- Content Rights: `No`
- Age Rating: questionnaire → `None` everywhere, except `Apps with User-Generated Content` → `Infrequent/Mild` (users write their own content)

### 7.4 Pricing and Availability

- Price: `Free (CHF 0.00 / EUR 0.00)`
- Availability: all countries (or restrict to EU+CH+US for staged rollout)

### 7.5 App Privacy (privacy nutrition labels)

- Privacy Policy URL: `https://mirkobozzetto.github.io/flowflow-privacy/` (from step 1)
- Data Practices → `Get Started`:

| Category | Collected | Linked to user | Tracking | Purpose |
|----------|-----------|----------------|----------|---------|
| Audio Data | Yes | No | No | App Functionality |
| User Content (notes) | Yes | No | No | App Functionality |
| Identifiers | No | — | — | — |
| Usage Data | No | — | — | — |
| Diagnostics | No | — | — | — |
| Contact Info | No | — | — | — |
| Financial Info | No | — | — | — |
| Health & Fitness | No | — | — | — |
| Purchases | No | — | — | — |
| Search History | No | — | — | — |

Save each section.

### 7.6 App Review (notes)

- Sign-In Required: `No`
- Reviewer Notes (English, copy-paste from `docs/deploy/02-app-store-submission.md` section 5d)
- Contact Info: Mirko Bozzetto, `mirko@mirko.re`, +32 484 906 499
- Demo Account: empty
- Additional Notes: empty

### 7.7 TestFlight Build

`Build` section → `Select a build` → choose `1.0 (1)`.

If nothing shows: wait for Apple processing to finish (step 5).

### 7.8 Export Compliance

- `Uses only standard iOS encryption?` → **Yes**

### 7.9 IDFA

- `App uses IDFA?` → **No**

---

## Step 8 — Submit for Review (1 min)

Left sidebar: all sections must have green ✓.

Top-right button: `Add for Review`.

Confirm → status moves to `Waiting for Review`.

Apple email on each transition:
- `Waiting for Review` → `In Review` (24-72h typical)
- `In Review` → `Ready for Distribution` or `Rejected`

If `Rejected`:
1. Read Apple feedback (email + ASC tab `App Review`).
2. Fix code or metadata.
3. Re-submit.

---

## Recommended execution order (parallelized)

```
[T+0]     Start Step 1 (privacy policy → GitHub Pages)         15 min
[T+0]     Start Step 2 (Soniox email)                           5 min   (parallel)
[T+15]    Start Step 3 (install Transporter)                   10 min
[T+25]    Start Step 4 (create ASC app)                        15 min
[T+40]    Start Step 5 (upload IPA Transporter)                 5 min + 30 min processing
[T+45]    Start Step 6 (screenshots) DURING processing         60 min
[T+105]   Start Step 7 (ASC metadata)                          60 min
[T+165]   Step 8 (submit)                                       1 min
[T+165]   Apple review wait                                     24-72h
```

Active total: ~2h45 + idle time.

---

## Final checklist before submit

- [ ] Privacy policy URL reachable (HTTP 200)
- [ ] Soniox DPA requested by email
- [ ] IPA `Delivered` + `Processed` in ASC
- [ ] 7 screenshots uploaded in correct order
- [ ] Description + keywords + subtitle filled
- [ ] Categories + age rating set
- [ ] Privacy nutrition labels complete
- [ ] App Review Notes in English filled
- [ ] TestFlight build selected in Version 1.0
- [ ] Export compliance = Yes
- [ ] IDFA = No
- [ ] `secrets/ios/distribution.key` backup confirmed (Google Drive ✓)

→ `Add for Review`.
