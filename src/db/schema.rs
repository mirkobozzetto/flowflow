pub const MIGRATIONS: &[(i64, &str)] = &[(1, V1_SCHEMA), (2, V2_SCHEMA)];

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
