# App Store Connect: "screenshots still uploading" that never clears

**Date:** 2026-05-29
**App:** FlowFlow (Apple ID 6773033233), iOS 1.0, first submission
**Status:** Resolved

## Symptom

Submission to review was blocked by a single error that would not go away:

> Impossible d'ajouter pour vérification — Des captures d'écran sont toujours en cours de chargement.
> (EN: "There are still screenshot uploads in progress.")

The iPhone 6.5" slot showed 4 clean screenshots (`4/10`). Nothing looked wrong in the UI.

## What we tried that did NOT work

- "Tout supprimer" + re-upload all screenshots.
- Re-upload one screenshot at a time with a page refresh between each.
- Waiting overnight (~12h+). The error survived a full day.
- Confirmed the local files were technically perfect: PNG, 1284×2778, sRGB IEC61966-2.1, **no alpha**.

The generic fixes from forums (delete + re-upload, wait, change browser) all failed because they never touched the actual blocker.

## Root cause (proven)

The web UI hid the problem. Querying Apple's internal **iris** API directly showed the `en-US` localization held **8** screenshot records, not 4:

```
en-US: 5 × COMPLETE  +  3 × FAILED
```

The 3 FAILED records, with Apple's own error code:

```
en01.png  e3ed4eea-829f-4b78-97f9-daf97e44d6da  FAILED  IMAGE_BAD_FILE_EXTENSION
en03.png  4f1165aa-6777-4445-8351-b705ba1eae21  FAILED  IMAGE_BAD_FILE_EXTENSION
en04.png  913778a9-dfe1-4a11-b263-76f1da808108  FAILED  IMAGE_BAD_FILE_EXTENSION
```

`IMAGE_BAD_FILE_EXTENSION` = the files were JPEG content saved with a `.png` name (the earlier resize-script bug). Apple rejected them server-side and marked them FAILED, but **never rendered them and never auto-deleted them**.

Why the loop never ended: the web Media Manager only draws COMPLETE screenshots (and grey placeholders for *some* in-progress states). FAILED records are invisible — no thumbnail, no placeholder, no red (-). So "Tout supprimer" only removed the visible COMPLETE ones; the 3 FAILED orphans stayed. But the submit-time validator counts every non-COMPLETE record → permanent block.

## The fix (reusable method, no API key needed)

Drive the App Store Connect internal **iris** API from the logged-in browser tab (uses the existing web session cookies — no `.p8` key, no app-specific password). Run in the browser DevTools console while on the App Store Connect page:

```js
// 1. find the in-flight version
const H = { Accept: 'application/vnd.api+json' };
const get = (u) => fetch(u, { credentials: 'include', headers: H }).then(r => r.json());

const v = await get('/iris/v1/apps/6773033233/appStoreVersions');
const vid = v.data.find(d => d.attributes.appStoreState === 'PREPARE_FOR_SUBMISSION').id;

// 2. list every localization (also exposes ghost/removed locales)
const locs = (await get(`/iris/v1/appStoreVersions/${vid}/appStoreVersionLocalizations`)).data;

// 3. per locale, inspect each screenshot's assetDeliveryState
for (const loc of locs) {
  const s = await get(`/iris/v1/appStoreVersionLocalizations/${loc.id}/appScreenshotSets?include=appScreenshots`);
  const bad = (s.included || []).filter(x =>
    x.type === 'appScreenshots' && x.attributes.assetDeliveryState.state !== 'COMPLETE');
  // 4. delete every non-COMPLETE (FAILED / stuck) record
  for (const b of bad) {
    await fetch(`/iris/v1/appScreenshots/${b.id}`, { method: 'DELETE', credentials: 'include', headers: H });
  }
}
```

`DELETE` returns HTTP `204`. After running it, re-read the sets: only COMPLETE remain, and the red submission banner disappears.

## Outcome

- `en-US` → 5 COMPLETE, 0 FAILED.
- Error banner gone; version back to clean `PREPARE_FOR_SUBMISSION`.
- Build 12 (DTXcodeBuild 17F42) attached and VALID.

## Takeaways

- "Screenshots still uploading" with a clean-looking UI usually means an **invisible FAILED/non-COMPLETE record**, often from a bad upload (JPEG-as-PNG → `IMAGE_BAD_FILE_EXTENSION`) or a removed localization.
- The web UI cannot delete what it does not render. The **iris API** is the escape hatch and needs no special credentials — just the browser session.
- Prevention: ensure screenshot files are real PNG (not JPEG renamed `.png`). `sips -g format <file>` must say `png`, and `file <file>` must say "PNG image data".
