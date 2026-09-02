# FlowFlow model

## Vocabulary

- A **space** is the server-scoped collaboration boundary. The bearer token
  fixes the only accessible space.
- A **folder** organizes notes. Folders form a parent-child hierarchy.
- A folder's `effective_mode` controls inherited collaboration behavior.
- A **note** belongs to one folder and has a title, body, author, sequence, and
  update time.
- A **thread** is a titled group of notes pinned to a folder. A note belongs
  to at most one thread. `read_thread` returns members in canonical order,
  oldest first by server creation time.
- `author_ref` identifies a human or agent author without exposing credentials.
- `own: true` means the note belongs to this exact agent identity.
- `seq` is the server change order. It is not a note creation date.
- `updated_at` is the timestamp currently exposed by the MCP server.

## Permission model

A `read` token can call all eight read actions. It cannot create, update, or
delete content.

A `read_write` token can also write below a folder whose `writable` value is
`true`. An agent cannot write at the space root. It can update or delete only
objects authored by its own durable agent identity.

Rotation changes the token but preserves that identity and its authorship.
Revocation invalidates every live token while preserving existing authorship.

FlowFlow presents one active Hermes integration per space. The backend can
represent multiple agent records, but this skill must use only the configured
server for the current space.

## Current capability boundary

The MCP server (`contract_version` 2) exposes folders, note metadata, note
bodies, threads with ordered membership, change cursors, and controlled note,
folder, and thread writes.

It does not currently expose:

- note creation timestamps distinct from `updated_at`;
- attachments;
- server-side full-text search;
- access to any space other than the token's space.

Never infer chronological creation order from titles, content, UUIDs, or `seq`.
State the missing capability when a request needs it. Never describe an
unavailable capability as a permission failure.

## Service limits

- Five live agents per space.
- Sixty agent writes per minute.
- Five hundred live folders per space.
- Five thousand live notes per space.
- One hundred rows per list or pull page.
- Note bodies are limited to 64 KiB.
- Folder depth is limited to eight.
- Agent tokens expire within 365 days.
