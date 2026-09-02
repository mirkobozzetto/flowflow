---
feature: "Hermes FlowFlow collaboration skill"
type: ship-contract
status: blocked
source_brief: docs/brief/hermes-flowflow-collaboration-skill/brief.md
---

# Delivery contract

## Deliverables

- A reusable Hermes skill installed from the FlowFlow repository.
- A safe first-run guide from access creation to the first note read.
- Operational guidance for search, analysis, controlled writes, and routines.
- Diagnostics that distinguish backend, authorization, and empty content states.
- A public FlowFlow guide linked from the repository documentation.
- A ten-scenario manual test plan covering Hermes, Telegram, and iPhone.

## Definition of done

The delivery is complete only when every acceptance criterion in `brief.md`
passes and all ten scenarios in `manual-test-plan.md` pass in one run.

The run must prove:

- all nine MCP actions;
- no access outside the token space;
- no write without an explicit instruction;
- no update or deletion of human-authored content;
- no duplicate after replaying a logical write;
- one visible copy of the written note on iPhone;
- active-channel confirmation without an implicit FlowFlow result note;
- no complete secret in output or evidence.

## Current blocker

The non-thread path is implemented, installed on the Hermes VPS, and verified
against the live MCP service with read-only actions.

The accepted brief also requires ordered thread reading. Current MCP responses
do not expose thread ids, titles, membership, or order. The skill states this
limit and refuses to infer topology. US5 and the 10/10 target remain blocked.

Full acceptance also requires the manual mutation, Telegram, routine, token
rotation, revocation, and iPhone scenarios. They change live external state or
need an explicit channel, timezone, and deletion confirmation.
