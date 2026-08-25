---
artifact: "docs/proposals/0003-space-pull-atomic-cursor-idempotent-publish/PROPOSAL.md"
stack: "rust / cargo (both repos)"
generated: "2026-08-25"
ran_by: "ship (safe set), user (deploy + device)"
---

# Verification Bundle: 0003 pull atomique + publish idempotent

## Safe checks (already run green by ship; rerun at will)

| Command | Validates | Expected pass signal |
|---------|-----------|----------------------|
| `cd ../marketplace-flowflow && cargo test --test spaces_test` | T01 C1-C5 | 24 passed |
| `cargo test --test space_delta_test --test space_publish_test --test space_schema_test --test space_leave_test` | T03-T08 C1-C6 | 13 + 3 + 2 + 4 passed |
| `make check` | fmt + clippy | no new warning |

## Destructive / stateful (USER ONLY)

| Step | Validates | Warning |
|------|-----------|---------|
| Merge backend PR, Dokploy deploy | T02: prod answers 201 then 200 on a replayed client id | outward-facing; MUST land before the next App Store build of the client |
| `make all` then device test | T09 | see below |

## Device script (T09)

1. Airplane mode. Open a collab folder, write a note, save. Back online, open the folder: exactly one server note, no duplicate, note keeps its content.
2. Kill the app in the middle of a large pull (join a space with many notes, force-quit during "pulling"). Reopen: no duplicate, nothing missing.
3. Save a note into a folder of a frozen space (owner unpaid): the note stays, retries later, is not detached.

## Contract coverage

- T01 C1-C5 -> `spaces_test` (3 new tests)
- T03-T08 C1-C4, C6 -> client tests above
- C5 detach on 400/404/409 and the real push -> device (T09), no HTTP mock in the client dev-deps
