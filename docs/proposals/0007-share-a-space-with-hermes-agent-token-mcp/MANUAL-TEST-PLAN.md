# Hermes manual test plan

## Current state

No phone or Hermes test is available yet.

T01 through T03 provide internal backend prerequisites. The MCP endpoint, owner
controls, app UI, and Hermes configuration are not complete.

## Delivery rule

A task is complete after its targeted verification and its own commit. A user
story is testable only after every required task is committed.

## Delivery sequence

1. Finish backend tasks T04 through T09.
2. Deploy the backend release containing the migration and MCP endpoint.
3. Finish app tasks T10 through T11.
4. Install that app build on the iPhone.

Deployment is the only external prerequisite before end-to-end testing.

## Owner flow on iPhone

1. Open a space owned by the premium account.
2. Open the Hermes panel in the space sidebar.
3. Create Hermes with `read_write` scope and copy its token immediately.
4. Create or select a `collab` folder for Hermes.
5. Confirm Hermes appears as an agent member.

Expected: the app never retains the token after its one-time display.

## Hermes flow on VPS

1. Confirm the deployed backend health endpoint returns HTTP 200 from the VPS.
2. Store the copied token in the Hermes environment file.
3. Configure one `flowflow_<space-slug>` MCP server for the deployed backend.
4. Reload Hermes and call `list_folders`.
5. Call `create_folder` and `put_note` with stable UUID v5 ids.
6. Repeat the same `put_note` call.
7. Call `ack_changes` only after the successful write.

Expected: Hermes sees the folders and the replay creates no duplicate note.

## Phone confirmation

1. Pull the space on the iPhone.
2. Confirm the Hermes folder and note appear once with an agent author marker.
3. Search that note from FlowFlow chat after embedding completes.
4. Revoke Hermes in the space panel.
5. Confirm the agent disappears from the member list.
6. Repeat one MCP call with the same token.

Expected: the revoked token returns HTTP 404.

## Scheduled confirmation

1. Enable one daily Hermes cron with only the FlowFlow MCP toolset.
2. Verify the cron gateway and timezone.
3. Wait for one scheduled execution, not a manual run.
4. Confirm its note arrives on the iPhone and advances `last_ack_seq`.
