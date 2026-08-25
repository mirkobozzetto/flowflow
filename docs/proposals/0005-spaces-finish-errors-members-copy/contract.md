---
artifact: "docs/proposals/0005-spaces-finish-errors-members-copy/PROPOSAL.md"
artifact_kind: "propose"
locked: "2026-08-25"
---

# Definition of Done: Spaces finish (errors, members, copy)

## Acceptance criteria (the contract)

| # | Criterion (from spec) | Source | Validated by |
|---|------------------------|--------|--------------|
| C1 | No `e.to_string()` on a `SpaceError` in `src/ui/sidebar` or `src/ui/settings/spaces.rs` | T01 | `grep -rn 'e.to_string()' src/ui/sidebar src/ui/settings/spaces.rs` |
| C2 | `error_key` has no wildcard arm, one distinct key per variant, keys exist in fr + en | T01 | `cargo test --test space_error_test` |
| C3 | Member without name shows `IconUserCircle` + 6-char handle | T02 | device |
| C4 | Copy link closes the panel and shows « Lien copié » | T02 | device |
| C5 | Opening any panel or the menu clears that line | T02 | device |
| C6 | Rename against an undeployed server shows a French message | T03 | device |

## Out of scope (never build)

- Translating inside `Display` (language is UI state)
- Hiding unnamed members
- `join_link.rs` keeps its own table

## Edit scope

- `src/application/space/mod.rs`, `src/ui/sidebar/space_section.rs`, `src/ui/settings/spaces.rs`, `fr.ftl`, `en.ftl`
- `folders.rs` error line: does not exist yet (0004 not shipped), nothing to wire
