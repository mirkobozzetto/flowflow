# Ship trace: thread note context menus

Proposal 0009 accepted by Mirko on 2026-09-08 with targeted tests and progressive
commit/push delivery. Solo execution. No public release or unrelated sync work.

## Source boundary

- Started on dev at 09e34a5 with preexisting deletion and sync/identity changes.
- Fetched origin/dev and origin/main. Fast-forwarded dev to 527bd1a: identical
  source tree, ancestry only. No branch switch or user-work stash.
- Publish the earlier, verified deletion correction as a prerequisite, without
  the unrelated sync/identity changes. Only its Result propagation hunk from
  space/pull.rs is staged; all identity changes remain local.
- Verify selected commit contents in a disposable detached worktree. The only
  ignored build input copied there is the required generated tailwind.css.

## Units

| Unit | State | Evidence |
| --- | --- | --- |
| P0: deletion prerequisite | Published: 487b803 | Isolated source: 37 tests pass across deletion, purge, space leave/publish, thread and sync suites. The extra local sync-repair test is deliberately excluded. |
| T1: stale menu-page regression | Reproduced | Real NoteRowMenu: abandoned Move/Delete confirmation, same/different note, all four reopenings fail before T2. Target: 1 passed, 1 failed. Retain this test with T2 to keep published commits green. |
| T2: session-owned note-menu page | Published: 54bea8f | The same four cases pass in the isolated staged source. Deletion target: 2 passed, 0 failed, including all earlier rollback/retry/navigation scenarios. Permanent root owner retained. GitNexus staged analysis: high, 7 flows; compiler and real integration exercised affected menu paths. |
| T3: thread Copy/Share gestures | Published: 90a22ba | Isolated staged source: 3 real ThreadNode event tests and 2 deletion/menu integration tests pass. Covers right-click, mouse/touch hold, release-click suppression, subsequent normal click, scroll/cancel/leave/unmount, two targeted note menus, Copy/Share only, return thread and unchanged content/membership. Clipboard OS write and real share-link presentation require native checks; no personal-note publication performed. |
| T4: targeted/non-regression checks | Passed: 45 tests | Clean published source 90a22ba: 3 gesture tests, 2 deletion/menu tests, 5 folder-tree, 4 pending-purge, 5 space-leave, 3 space-publish, 12 sync-protocol and 11 thread tests. No failures. No personal data used or published. Native checks accompany T5. |
| T5: installed apps/manual handoff | Completed | Mac native checks passed on isolated source 90a22ba. Initial iPhone installation was blocked by DeviceLocked; two later user-run make all commands installed successfully on 2026-09-08 at 14:06:13 and 14:06:51. Mirko reports the changes appear to work on mobile. These final phone builds include the regular checkout’s unrelated local changes; they are not clean release artifacts. |

GitNexus detect-changes executed for all and staged scopes. Staged prerequisite
reports critical (21 symbols, 67 processes), including deletion/space callers.
Impact traversal remains degraded/UNKNOWN; no graph all-clear. Exact staged
source and the targeted integration executions provide the behavioral evidence.

## Native evidence and remaining installation

- Binary source: `90a22ba` (unrelated local sync/identity work excluded).
- The installed Mac binary was copied unchanged into a disposable native run.
  Outbound Internet access was denied; no personal note was published.
- Actual macOS right-click and long-press exposed only Copy text and Share this
  note. Both abandoned Move and Delete confirmation reopened on the actions.
- Native clipboard matched the full selected note, including Unicode and
  newlines: 1,215 UTF-8 bytes, not the truncated card preview.
- Sharing reused the selected note’s fixture link, not the other note or the
  whole thread. Back returned to the thread; an ordinary click opened the
  selected note editor. New remote publication was not performed.
- Confirmed native deletion removed only the selected disposable root note.
- The smoke app was closed. Original clipboard items and types were restored
  and compared successfully. The user’s running Mac process was not closed;
  quit and reopen FlowFlow from Applications to load the installed binary.

Installed Mac executable SHA-256:
`2d11c6ab1b2241f589ae6b5beb4c2052a6813d6b550d2646e3fb95de008d24be`

Prepared iPhone executable SHA-256:
`94fac52b4800f1112b9c928fa98ef067bbe347dbe1e43bde32844c07da14f0ea`

At the initial handoff, the iPhone build was **not installed**. `make all` masked the final
installer’s failure with `|| true`; its zero exit is not installation proof.
The actual installer reported `kAMDMobileImageMounterDeviceLocked`.
At 13:21 on 2026-09-08, `devicectl device info lockState` still reported
`passcodeRequired: true`. The initial install-only handoff was:

```sh
xcrun devicectl device install app \
  --device 74506719-A175-5649-AAB8-7DDC1710664D \
  /Users/mirkobozzetto/code/flowflow/target/dx/flowflow/debug/ios/Flowflow.app
```

## Build recovery after the main-checkout failure

- Root cause proved from the user’s terminal history: Whisper CMake failed
  because its cache recorded the disposable worktree path, while the same
  target directory was subsequently opened from the regular checkout.
  This was caused by the agent sharing target through a worktree symlink.
- Regenerated only the affected Whisper CMake build directory. No worktree
  was created for the repair; unrelated working-tree changes were retained.
- make all now preserves the ios-dev/desktop-dev Cargo profiles, propagates
  URL-scheme/icon/signing/installation failures, and selects a paired physical
  iOS device from devicectl JSON rather than its changing display status.
- Actual make all from the regular checkout compiled successfully in 97.29 s;
  the next incremental build completed in 11.20 s. Signing and deep strict
  codesign verification passed. Installation was still unverified at this
  stage: CoreDevice returned Failed to acquire assertion with the phone locked.
  The successful user-run installations below supersede that blocker.
- Current prepared iPhone executable SHA-256:
  `a90ff5b4c081cd669071f125e5990e1de8861f5007ae4ea47577d2f2145d6517`.
- Global rules now require explicit approval before each worktree, isolated
  or verified canonical build-output paths, and build/install evidence before
  announcing readiness. Dependency download caches are not prohibited.

## Successful iPhone installation

- Read Mirko’s actual Herdr command history, not just the Make exit status.
  Both later runs reported `App installed:` for
  `com.mirkobozzetto.flowflow`, followed by `>> Done.` and the shell prompt.
- Installation times: 2026-09-08 at 14:06:13 and 14:06:51. Mirko reports the
  changes appear to work. No additional build was launched to reconfirm him.
- All six progressive commits through `2fcdd64` are present on origin/dev.
  Personal data and unrelated sync/identity work remain untouched.

## Release PR and App Store handoff

Authorized scope: create the dev-to-main PR and prepare the following delivery
steps. No merge, release publication, Apple upload or review submission.

### Baseline and next version

- Public iOS version verified through Apple’s Belgium lookup endpoint:
  **2.0.2**, released **2026-09-07T21:29:34Z**.
  <https://itunes.apple.com/lookup?id=6773033233&country=be>
- Last previously verified upload: **build 16**, from source `527bd1a`. The
  existing release worktree retains its generated 2.0.2/build-16 counters.
  Restore those counters in version control, including Cargo.lock, so the
  existing Makefile computes **2.0.3/build 17** instead of reusing 2.0.2/16.
- Build 17 is provisional until App Store Connect confirms no newer upload.
  Its current private state could not be read: Helium CDP on port 9222 was
  unavailable. No browser session was closed or relaunched.
- Public desktop baseline: **v2.0.1**, source `527bd1a`, published 2026-09-07.
  Compare actual source content, not historical commit counts: previous squash
  merges leave already-released commits in the dev-to-main ancestry.

### Delivery sequence, not executed

1. Review the PR and authorize its merge with a **merge commit**, not squash
   or rebase. The current PR includes only committed work, not the local
   sync/identity repair, portrait settings or screenshot-workflow edits.
2. Confirm App Store Connect’s latest build, all existing localizations and
   review state. Reconfirm the public build/source mapping before fixing the
   final release baseline.
3. Prepare a clean checkout of the approved candidate. Both existing
   checkouts currently contain local changes; do not stash or reuse them
   silently. Any new worktree needs Mirko’s explicit approval and its own
   path-sensitive build outputs. Never share target through a symlink.
4. Build and validate with `make appstore APPSTORE_VERSION=2.0.3
   APPSTORE_BUILD=17` only after confirming those values. Unlike `make ship-ios`,
   this target does not upload. Record the exact candidate, generated version
   changes, IPA SHA-256, bundle/extension versions, distribution signatures
   and the actual Apple validator result. No new distributable exists yet.
5. Prepare the complete metadata and screenshot package before proposing an
   upload: audit every current localization, descriptions, promotional text,
   keywords, URLs, privacy, permissions and review instructions. Inventory the
   current screenshots and convert the selected originals with the existing
   `make appstore-screenshots` workflow, verifying every final dimension.
6. Present that complete package for approval. Upload only after authorization;
   verify processing and attach the new build. Review submission requires a
   separate explicit approval.

### Draft patch release notes

English:
- Copy or share an individual note directly from a thread with a long press.
- Note menus reopen on their main actions after Move or Delete is dismissed.
- More reliable note deletion, with clear errors and safe recovery on failure.

French:
- Copiez ou partagez une note d’un fil directement par un appui long.
- Les menus reviennent aux actions principales après la fermeture des écrans
  de déplacement ou de suppression.
- La suppression des notes est plus fiable, avec des erreurs explicites et
  une récupération sûre en cas d’échec.

These are draft release notes, not an audited or submitted metadata package.


