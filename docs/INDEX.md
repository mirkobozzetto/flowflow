# FlowFlow documentation index

> The README is the showcase; this is the map. One line per document, grouped by chapter.

## 01 Product

- [README](../README.md) - pitch, features, stack, quick start
- [HISTORY](HISTORY.md) - chronological log of every shipped milestone
- [Roadmap](roadmap/) - what comes next (post-RFC-0004 plan)
- [Stories](stories/) - debugging war stories worth keeping

## 02 Architecture

- [CLAUDE.md](../CLAUDE.md) - the living architecture reference: modules, data entities, pipelines, stack versions
- [Brainstorm](brainstorm/) - deep dives and explorations (system audio, imports)

## 03 Dev guides

- Build and run: see [README - Setup](../README.md#setup) and [README - Commands](../README.md#commands)
- Physical device setup (one-time pairing, certificates): [CLAUDE.md - Physical Device Setup](../CLAUDE.md)
- Tests: `cargo test` (208 tests), `cargo test -- --ignored` for API-key-gated ones

## 04 App Store

- [App Store guide](guides/appstore.md) - provisioning, signing, IPA, submission, troubleshooting, quick links
- [Deploy notes](deploy/) - fresh setup, submission walkthrough, execution plan, Dioxus workarounds

## 05 Specs

- [RFCs](rfcs/) - technical designs: 0001 data backup/export, 0004 multidevice sync
- [PRDs](prd/) - product specs: multidevice-sync, sync-realtime-ux, data-backup-export

## 06 History

- [HISTORY.md](HISTORY.md) - everything that shipped, by milestone, with links

---

## Doc rules (how to keep this readable)

1. The README is a showcase: hard ceiling ~100 lines, no ops content, no troubleshooting.
2. Anything longer goes into `docs/` and gets ONE line in this index, in the right chapter.
3. `HISTORY.md` gets one entry per significant merged PR (3-4 lines, date + links). Update it in the same PR.
4. Never duplicate content between README, INDEX, and guides; link instead.
5. New chapter only when an existing one clearly does not fit. Prefer fewer, fuller chapters.
