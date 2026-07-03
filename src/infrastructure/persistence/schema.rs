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
];

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
