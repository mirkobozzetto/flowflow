# Preferences and routines

FlowFlow knowledge and user preferences are separate.

## Preferences

When the user defines preferred folders, subjects, or outputs:

1. Restate the proposed preferences before first use.
2. Apply them only after the user confirms them.
3. Keep them outside this skill and outside FlowFlow notes by default.
4. Change or stop them immediately when asked.
5. Never let a preference expand the token's space or write permissions.

## Routines

No routine is active by default. Create one only after an explicit request.
Before creating it, state:

- the exact space server;
- the visible folder or note scope;
- the schedule and timezone;
- whether it reads, writes, or both;
- the expected result and confirmation channel;
- the stable UUID strategy for every repeated write.

A routine must use only the matching FlowFlow MCP toolset. It must follow the
same ownership, confirmation, idempotency, and cursor rules as an interactive
request.

The user must be able to inspect, change, suspend, and stop every routine.
Stopping a routine does not revoke the FlowFlow token. Revocation happens in
the FlowFlow owner panel.

## Manage routines with Hermes cron

Create a routine only after the user confirms the full definition. Before
creation, verify the scheduler timezone. Set `HERMES_TIMEZONE` to the confirmed
IANA timezone and reload Hermes, or convert the schedule to the active timezone.
Do not create a routine when daylight-saving behavior is ambiguous.

For Telegram delivery, run `/sethome` in the intended chat first. An explicit
`telegram:<chat-id>` target is also valid. Then create the routine:

```sh
hermes cron create --name flowflow-review --skill flowflow-spaces --deliver telegram "0 9 * * *" "Use only flowflow_space. <task>"
```

Use the returned job id for every lifecycle action:

```sh
hermes cron list --all
hermes cron edit <job-id> --schedule "0 10 * * *"
hermes cron pause <job-id>
hermes cron resume <job-id>
hermes cron runs <job-id>
hermes cron remove <job-id>
```

The prompt must name the MCP server, target, allowed writes, output, and
timezone. Plain `hermes cron list` shows active jobs only.

Telegram is only a delivery channel. If the request arrived through an active
Telegram conversation, the normal reply confirms the result there. Do not
create an extra FlowFlow note unless the user explicitly asks for one.
