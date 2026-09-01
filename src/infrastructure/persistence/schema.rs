pub const MIGRATIONS: &[(i64, &str)] = &[
    (1, V1_SCHEMA),
    (2, V2_SCHEMA),
    (3, V3_SCHEMA),
    (4, V4_SCHEMA),
    (5, V5_SCHEMA),
    (6, V6_SCHEMA),
    (7, V7_SCHEMA),
    (8, V8_SCHEMA),
    (9, V9_SCHEMA),
    (10, V10_SCHEMA),
    (11, V11_SCHEMA),
    (12, V12_SCHEMA),
    (13, V13_SCHEMA),
    (14, V14_SCHEMA),
    (15, V15_SCHEMA),
    (16, V16_SCHEMA),
    (17, V17_SCHEMA),
    (18, V18_SCHEMA),
    (19, V19_SCHEMA),
    (20, V20_SCHEMA),
    (21, V21_SCHEMA),
    (22, V22_SCHEMA),
    (23, V23_SCHEMA),
    (24, V24_SCHEMA),
    (25, V25_SCHEMA),
    (26, V26_SCHEMA),
    (27, V27_SCHEMA),
    (28, V28_SCHEMA),
];

pub enum TableClass {
    Content,
    SyncState,
    DeviceLocal,
    Config,
    Internal,
}

pub const TABLES: &[(&str, TableClass)] = &[
    ("note_audio_words", TableClass::Content),
    ("note_shares", TableClass::Content),
    ("note_provenance", TableClass::Content),
    ("note_audios", TableClass::Content),
    ("attachments", TableClass::Content),
    ("note_reminders", TableClass::Content),
    ("note_links", TableClass::Content),
    ("conversation_messages", TableClass::Content),
    ("conversations", TableClass::Content),
    ("notes_folders", TableClass::Content),
    ("chunks", TableClass::Content),
    ("spaces", TableClass::Content),
    ("pending_purge", TableClass::Content),
    ("space_publish_pending", TableClass::Content),
    ("pending_transcriptions", TableClass::Content),
    ("notes", TableClass::Content),
    ("threads", TableClass::Content),
    ("folders", TableClass::Content),
    ("sync_row_meta", TableClass::SyncState),
    ("sync_seq", TableClass::SyncState),
    ("sync_conflicts", TableClass::SyncState),
    ("sync_peers", TableClass::DeviceLocal),
    ("settings", TableClass::Config),
    ("installed_agents", TableClass::Config),
    ("installed_connectors", TableClass::Config),
    ("_migrations", TableClass::Internal),
];

const V28_SCHEMA: &str = "
ALTER TABLE chunks ADD COLUMN embed_profile TEXT NOT NULL
    DEFAULT 'openai:text-embedding-3-small:1536';
";

// A note saved into a space folder is bound to its remote id and queued here
// until the server has it; the queue is drained, bounded, at the head of every
// pull. DEVICE-LOCAL like `spaces` (absent from sync/protocol/catalog.rs): a
// publication is owed by the device that wrote the note, and a peer's
// INSERT OR REPLACE must never rewrite its retry state.
const V27_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS space_publish_pending (
    note_id TEXT PRIMARY KEY,
    space_id TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_try_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    last_error TEXT
);
CREATE INDEX IF NOT EXISTS idx_publish_pending_due
    ON space_publish_pending(space_id, next_try_at);
";

// Collaborative spaces (proposal 0002). `spaces` is the per-device pull cursor
// for a space the user joined, `pending_purge` the replayable intent to drop a
// note's vectors from LanceDB. Both are DEVICE-LOCAL: absent from
// sync/protocol/catalog.rs, so no trigger is generated and they never reach the
// wire. A cursor is meaningless on another device, and a purge intent belongs to
// whichever device still holds the vector.
//
// The columns added to `folders` and `notes` DO travel (declared in catalog.rs):
// a space folder must look the same on every device of one account. `mode` is
// the DECLARED mode; the effective mode is resolved by walking the ancestor
// chain, never stored. NO foreign key to `spaces`, same trap as V20/V25: the
// sync applier upserts with INSERT OR REPLACE and a cascade would wipe the rows
// on every peer echo. Cleanup is manual in the delete path.
const V26_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS spaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    is_owner INTEGER NOT NULL DEFAULT 0,
    joined_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    cursor INTEGER NOT NULL DEFAULT 0,
    last_pull_at TEXT
);

CREATE TABLE IF NOT EXISTS pending_purge (
    note_id TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'note'
        CHECK (kind IN ('note', 'attachment')),
    queued_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (note_id, kind)
);

ALTER TABLE folders ADD COLUMN space_id TEXT;
ALTER TABLE folders ADD COLUMN remote_id TEXT;
ALTER TABLE folders ADD COLUMN mode TEXT;

ALTER TABLE notes ADD COLUMN space_id TEXT;
ALTER TABLE notes ADD COLUMN remote_id TEXT;
ALTER TABLE notes ADD COLUMN author_ref TEXT;

CREATE INDEX IF NOT EXISTS idx_folders_space ON folders(space_id);
CREATE INDEX IF NOT EXISTS idx_notes_space ON notes(space_id);
CREATE INDEX IF NOT EXISTS idx_notes_remote ON notes(remote_id);
CREATE INDEX IF NOT EXISTS idx_folders_remote ON folders(remote_id);
";

// Shared notes/threads (proposal 0001). note_shares = MY live publications,
// keyed by the LOCAL source id (note or thread; republish replaces the row);
// note_provenance = the frozen origin of a note KEPT from someone else's
// share, keyed by the local note id, plus its alignment state ('live' aligns
// with the backend, 'gone' greys the author out). Both travel in sync
// (cluster devices see the same shares) and in backup. NO foreign key on
// notes: the sync applier upserts notes with INSERT OR REPLACE, and an FK
// cascade would wipe these rows every time a peer echoed a note update (same
// trap documented on V20). Cleanup is manual in the note delete path. The PK
// is named `id` because the sync catalog locates every non-link row by it.
const V25_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS note_shares (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('note', 'thread')),
    code TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    modified_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS note_provenance (
    id TEXT PRIMARY KEY,
    share_code TEXT NOT NULL,
    remote_note_id TEXT NOT NULL,
    author_name TEXT,
    captured_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    state TEXT NOT NULL DEFAULT 'live'
        CHECK (state IN ('live', 'gone')),
    modified_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
";

// Human-readable peer label exchanged in the sync HELLO (protocol v4). A
// label for the pairing UI and author chips, never an identity: authorization
// stays key-based.
const V24_SCHEMA: &str = "
ALTER TABLE sync_peers ADD COLUMN name TEXT;
";

// Who wrote the note: the local sync_device_id (a UUID minted at first launch,
// no backend needed). Nullable; pre-V23 rows are backfilled by the migrate hook
// from sync_row_meta.origin_device (last-writer, an accepted one-time
// approximation), local-only rows fall back to the local device id. The
// backfill MUST run under set_applying: the unqualified AFTER UPDATE sync
// trigger on notes would otherwise re-author every row and re-push the corpus
// to every peer.
const V23_SCHEMA: &str = "
ALTER TABLE notes ADD COLUMN author_device TEXT;
";

// Per-word timings and confidence for a transcription (RFC 0024). Device-local: it
// is absent from sync/protocol/catalog.rs and from sync_meta's Tracked list, so no
// trigger is created and it never reaches the wire.
//
// A separate table rather than a column on note_audios, because the sync applier
// upserts with INSERT OR REPLACE (apply/entity.rs) and note_audios is declared
// update_trigger: true - a column here would be wiped every time a peer echoed a
// transcription update back. No foreign key for the same reason: the row deletion
// inside INSERT OR REPLACE would fire ON DELETE CASCADE and reintroduce the wipe.
//
// Integrity is therefore manual, and the enumeration has to stay complete: four
// paths remove a note_audios row, and wipe_local_content's hardcoded table list is
// one of them - a transcript surviving "delete my data" is a privacy defect.
const V20_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS note_audio_words (
    audio_id TEXT PRIMARY KEY,
    words_json TEXT NOT NULL
);
";

// Carries the recorded clip through a durable transcription job: a recording
// keeps its `note_audios` row, so its word timings have an anchor and the file
// must survive the job. An import leaves it NULL.
const V21_SCHEMA: &str = "
ALTER TABLE pending_transcriptions ADD COLUMN audio_id TEXT;
";

// Per-question RAG execution trace, persisted with the bot message. Nullable and
// device-local: the sync wire never carries it, so a plain additive column suffices
// (triggers survive ALTER TABLE ADD COLUMN).
const V19_SCHEMA: &str = "
ALTER TABLE conversation_messages ADD COLUMN trace_json TEXT;
";

// Reminder cards persist as messages with role "reminder", which the v18 CHECK does not
// admit: the insert failed, the card lived only in memory, and the first reload wiped it -
// buttons appearing then vanishing. Same rebuild-and-copy as v18 (SQLite cannot alter a
// CHECK in place), keeping trace_json added since, and the triggers dropped with the table
// are reinstalled by the v22 migrate hook.
const V22_SCHEMA: &str = "
CREATE TABLE conversation_messages_new (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL
        REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL
        CHECK (role IN ('user', 'bot', 'proposal', 'reminder')),
    content TEXT NOT NULL,
    sources_json TEXT,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    trace_json TEXT
);
INSERT INTO conversation_messages_new
    (id, conversation_id, role, content, sources_json, created_at, trace_json)
    SELECT id, conversation_id, role, content, sources_json, created_at, trace_json
    FROM conversation_messages;
DROP TABLE conversation_messages;
ALTER TABLE conversation_messages_new RENAME TO conversation_messages;
CREATE INDEX IF NOT EXISTS idx_cm_conversation
    ON conversation_messages(conversation_id);
";

// Approval cards (RFC 0019) persist as messages with role "proposal", but the original
// conversation_messages CHECK only admits 'user'/'bot'. SQLite cannot alter a CHECK in place,
// so rebuild the table with the widened constraint and copy the rows. The sync triggers bound
// to the old table are dropped with it and reinstalled by the v18 migrate hook.
const V18_SCHEMA: &str = "
CREATE TABLE conversation_messages_new (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL
        REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'bot', 'proposal')),
    content TEXT NOT NULL,
    sources_json TEXT,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO conversation_messages_new
    (id, conversation_id, role, content, sources_json, created_at)
    SELECT id, conversation_id, role, content, sources_json, created_at
    FROM conversation_messages;
DROP TABLE conversation_messages;
ALTER TABLE conversation_messages_new RENAME TO conversation_messages;
CREATE INDEX IF NOT EXISTS idx_cm_conversation
    ON conversation_messages(conversation_id);
";

// Pinned, signed connector manifests (RFC 0016): the (resource, action, risk) classification the
// gate applies, verified against the pinned admin key before pinning - same trust regime as
// installed_agents. `version` backs the anti-rollback check; type resolution takes the lowest slug
// among pins of a type (the shared deterministic order). Backfilled with the compiled Sheets
// fixture by the v17 migrate hook so existing installs keep working offline.
const V17_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS installed_connectors (
    slug TEXT PRIMARY KEY,
    connector_type TEXT NOT NULL,
    version INTEGER NOT NULL,
    content_digest TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    pinned_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_installed_connectors_type
    ON installed_connectors(connector_type, slug);
";

// Persistent note-links graph (issue #90). LOCAL and never synced: links are
// derivable from chunks, so each device computes its own (no version vectors,
// no tombstones, no wire changes). 'dismissed' rows survive recomputes so a
// rejected link never comes back; 'pinned' rows always rank first.
const V16_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS note_links (
    src_note_id TEXT NOT NULL
        REFERENCES notes(id) ON DELETE CASCADE,
    dst_note_id TEXT NOT NULL
        REFERENCES notes(id) ON DELETE CASCADE,
    score REAL NOT NULL DEFAULT 0,
    label TEXT,
    state TEXT NOT NULL DEFAULT 'active'
        CHECK (state IN ('active', 'dismissed', 'pinned')),
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (src_note_id, dst_note_id)
);
CREATE INDEX IF NOT EXISTS idx_note_links_dst
    ON note_links(dst_note_id);
";

// Pinned, signed agents (RFC 0010). `manifest_json` is the canonical-JSON form the pinned
// `content_digest` was computed over: the row is verified by recomputing that digest, never
// re-interpreted by an LLM. `active` reflects the account link; an agent can be installed but off.
const V14_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS installed_agents (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    content_digest TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    installed_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    active INTEGER NOT NULL DEFAULT 1
);
";

// Per-install resource binding chosen at arm time (RFC 0010). NULL = unbound, so the manifest's
// placeholder bound stands and off-bound writes stay refused. Survives a re-pin: install_agent never
// touches this column, so updating an agent keeps the sheet the user armed it to.
const V15_SCHEMA: &str = "
ALTER TABLE installed_agents ADD COLUMN bound_json TEXT;
";

const V12_SCHEMA: &str = "
ALTER TABLE notes ADD COLUMN sources_json TEXT;
";

const V13_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS threads (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL DEFAULT '',
    folder_id TEXT,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    modified_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_threads_folder ON threads(folder_id);

ALTER TABLE notes ADD COLUMN thread_id TEXT REFERENCES threads(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_notes_thread ON notes(thread_id);
";

const V1_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    note_type TEXT NOT NULL DEFAULT 'voice'
        CHECK (note_type IN ('voice', 'text')),
    title TEXT,
    content TEXT NOT NULL DEFAULT '',
    audio_file_path TEXT,
    duration_secs REAL,
    tags TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    modified_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS folders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    parent_id TEXT,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    modified_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (parent_id) REFERENCES folders(id)
        ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_folders_parent
    ON folders(parent_id);

CREATE TABLE IF NOT EXISTS notes_folders (
    folder_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (folder_id, note_id),
    FOREIGN KEY (folder_id) REFERENCES folders(id)
        ON DELETE CASCADE,
    FOREIGN KEY (note_id) REFERENCES notes(id)
        ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_nf_folder
    ON notes_folders(folder_id);
CREATE INDEX IF NOT EXISTS idx_nf_note
    ON notes_folders(note_id);
";

const V2_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    modified_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS conversation_messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL
        REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'bot')),
    content TEXT NOT NULL,
    sources_json TEXT,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_cm_conversation
    ON conversation_messages(conversation_id);
";

const V3_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL
        REFERENCES notes(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    content_text TEXT NOT NULL DEFAULT '',
    imported_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_attachments_note
    ON attachments(note_id);
";

const V4_SCHEMA: &str = "";

const V5_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS note_audios (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    duration_secs REAL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_note_audios_note ON note_audios(note_id);

INSERT INTO note_audios (id, note_id, file_path, duration_secs, created_at)
SELECT lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
       substr(hex(randomblob(2)),2) || '-' ||
       substr('89ab', abs(random()) % 4 + 1, 1) ||
       substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
       id, audio_file_path, duration_secs, created_at
FROM notes WHERE audio_file_path IS NOT NULL;
";

const V6_SCHEMA: &str = "
ALTER TABLE note_audios ADD COLUMN transcription TEXT;
";

const V7_SCHEMA: &str = "
ALTER TABLE notes DROP COLUMN audio_file_path;
ALTER TABLE notes DROP COLUMN duration_secs;
";

const V8_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS pending_transcriptions (
    note_id TEXT PRIMARY KEY,
    transcription_id TEXT NOT NULL,
    soniox_file_id TEXT,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
";

const V9_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS note_reminders (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL
        REFERENCES notes(id) ON DELETE CASCADE,
    reminder_id TEXT NOT NULL,
    backend TEXT NOT NULL
        CHECK (backend IN ('eventkit', 'usernotifications')),
    intent_hash TEXT NOT NULL,
    due_year INTEGER,
    due_month INTEGER,
    due_day INTEGER,
    due_hour INTEGER,
    due_minute INTEGER,
    is_all_day INTEGER NOT NULL DEFAULT 0,
    tz_id TEXT,
    recurrence TEXT,
    state TEXT NOT NULL DEFAULT 'active'
        CHECK (state IN ('active', 'tombstone')),
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE(note_id, intent_hash)
);
CREATE INDEX IF NOT EXISTS idx_note_reminders_note
    ON note_reminders(note_id);
";

// V10: multi-device sync foundation (RFC 0004). PURELY ADDITIVE.
// Tables only. The INSERT/UPDATE tracking triggers are generated and installed
// by the V10 migrate hook (db::sync_meta::install_sync_triggers) so the trigger
// bodies stay DRY and reviewable instead of being hand-written N times here.
// V1-V9 are never touched; CASCADE FKs are preserved (deletes are tombstoned
// applicatively, cf. db::sync_meta).
const V10_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sync_row_meta (
    entity_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    version_vector TEXT NOT NULL DEFAULT '{}',
    origin_device TEXT NOT NULL,
    origin_seq INTEGER NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0,
    updated_hlc TEXT,
    PRIMARY KEY (entity_kind, entity_id)
);
CREATE INDEX IF NOT EXISTS idx_sync_meta_origin
    ON sync_row_meta(origin_device, origin_seq);
CREATE INDEX IF NOT EXISTS idx_sync_meta_deleted
    ON sync_row_meta(deleted);

CREATE TABLE IF NOT EXISTS sync_seq (
    device_id TEXT PRIMARY KEY,
    next_seq INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sync_peers (
    device_id TEXT PRIMARY KEY,
    static_pubkey TEXT,
    last_acked_seq INTEGER NOT NULL DEFAULT 0,
    paired_at TEXT,
    gc_horizon INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sync_conflicts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    losing_vv TEXT NOT NULL,
    losing_snapshot_json TEXT NOT NULL,
    losing_vector_ref TEXT,
    created_hlc TEXT,
    resolved INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_sync_conflicts_unresolved
    ON sync_conflicts(resolved);

CREATE TABLE IF NOT EXISTS chunks (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    owner_kind TEXT NOT NULL CHECK (owner_kind IN ('note', 'attachment')),
    chunk_index INTEGER NOT NULL,
    dim INTEGER NOT NULL DEFAULT 1536,
    vector BLOB NOT NULL,
    content_hash TEXT,
    chunk_text TEXT,
    title TEXT,
    tags TEXT,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_chunks_owner
    ON chunks(owner_id);
";

const V11_SCHEMA: &str = "
CREATE TABLE pending_transcriptions_v11 (
    note_id TEXT PRIMARY KEY,
    transcription_id TEXT,
    soniox_file_id TEXT,
    provider TEXT NOT NULL DEFAULT 'soniox',
    file_path TEXT,
    created_at TEXT NOT NULL
        DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
INSERT INTO pending_transcriptions_v11
    (note_id, transcription_id, soniox_file_id, created_at)
SELECT note_id, transcription_id, soniox_file_id, created_at
FROM pending_transcriptions;
DROP TABLE pending_transcriptions;
ALTER TABLE pending_transcriptions_v11 RENAME TO pending_transcriptions;
";
