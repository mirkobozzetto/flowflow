pub mod folder_repo;
pub mod note_repo;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

const MIGRATIONS: &[(i64, &str)] = &[(1, V1_SCHEMA)];

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

pub fn db_path() -> PathBuf {
    #[cfg(target_os = "ios")]
    {
        let dir = crate::platform::ios::documents_dir();
        std::fs::create_dir_all(&dir).ok();
        dir.join("flowflow.db")
    }
    #[cfg(not(target_os = "ios"))]
    {
        let dir = std::env::temp_dir().join("flowflow");
        std::fs::create_dir_all(&dir).ok();
        dir.join("flowflow.db")
    }
}

pub fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

impl Database {
    pub fn open() -> Result<Self, String> {
        let path = db_path();
        eprintln!("[db] opening {}", path.display());
        let conn =
            Connection::open(&path).map_err(|e| format!("DB open: {e}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("WAL: {e}"))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("FK: {e}"))?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        eprintln!("[db] ready");
        Ok(db)
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
                    DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
            );",
        )
        .map_err(|e| format!("Migration table: {e}"))?;

        let current: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _migrations",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Version check: {e}"))?;

        for &(version, sql) in MIGRATIONS {
            if version > current {
                eprintln!("[db] applying migration v{version}");
                conn.execute_batch(sql)
                    .map_err(|e| format!("Migration v{version}: {e}"))?;
                conn.execute(
                    "INSERT INTO _migrations (version) VALUES (?1)",
                    [version],
                )
                .map_err(|e| format!("Record v{version}: {e}"))?;
            }
        }
        Ok(())
    }
}
