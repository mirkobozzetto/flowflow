# Prompt 3 - Upgrade Dioxus 0.7 to 0.7.9

Self-contained task block. Estimated time: 30 minutes. Standalone (no prerequisites).

## Preflight

Run before any edit:

```bash
bash .claude/skills/ios-background-audio/scripts/check-dx-version.sh
```

If `dx --version` already reports 0.7.9, skip the upgrade and jump to step 4 (verify build).

## Block

```
<context>
FlowFlow uses Dioxus 0.7.0.
Dioxus 0.7.9 is the latest stable release (2026-05-08).
0.7.0 -> 0.7.9 is semver-compatible, no breaking changes.
The dx CLI must match the library version.

Key releases:
- 0.7.4: iOS Widget Extensions (PR #4842), Swift FFI, Dioxus.toml iOS config
- 0.7.5: futures dep fix (0.3.32 required)
- 0.7.6: last feature release for 0.7
- 0.7.7 to 0.7.9: macOS signing fix, iframe fix, dx fixes

Current Cargo.toml: dioxus = { version = "0.7" }
Current dx CLI: 0.7.7
</context>

<task>
Upgrade Dioxus to 0.7.9.

1. cargo update (resolves dioxus 0.7.9 automatically via the "0.7" semver range)
2. dx self-update (or cargo install dioxus-cli@0.7.9 --force)
3. Verify dx --version reports 0.7.9
4. make build (cargo build --features mobile)
5. If compile errors appear: analyse and fix (likely none for a patch bump)
6. make check (fmt + clippy)
7. Test on simulator: make dev
8. Test on device: make ddev
</task>

<constraints>
- Do not jump to 0.8.0-alpha (unstable)
- If a crate has a version conflict, resolve it inside Cargo.toml
- Verify futures 0.3.32 is the resolved version (required since 0.7.5)
- Do not commit without explicit user approval
</constraints>

<success_criteria>
- cargo build --features mobile compiles with no errors
- dx --version is 0.7.9
- make check is clean
- App runs on simulator and on device
</success_criteria>
```

## Notes

- This upgrade unblocks Prompt 4 (Dynamic Island Live Activity) because Widget Extension support landed in Dioxus 0.7.4 (PR #4842).
- After `cargo update`, inspect `Cargo.lock` for the resolved `dioxus` and `futures` versions before running `make build`. Quick check: `grep -E '^name = "(dioxus|futures)"' Cargo.lock`.
- The user runs `make build`, `make dev`, and `make ddev`. You only run `cargo update`, `dx self-update`, and `make format && make check`.

## See also

- `scripts/check-dx-version.sh` for the version probe
- `references/prompt-4-dynamic-island.md` for the consumer of this upgrade
