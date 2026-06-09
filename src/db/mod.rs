pub mod attachment_repo;
pub mod chunk_repo;
pub mod conversation_repo;
pub mod folder_repo;
pub mod note_reminder_repo;
pub mod note_repo;
pub mod pending_transcription_repo;
mod schema;
pub mod settings_repo;
pub mod sync_meta;

use rusqlite::Connection;
use schema::MIGRATIONS;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Mutex<Connection>,
    // Connection-local "applying a remote batch" flag (RFC 0004). Read by the
    // tracking triggers via the registered SQL function sync_is_applying(): when
    // true, the triggers no-op so the sync service can write meta verbatim. Only
    // THIS Database's connection sees it, so concurrent local writes on other
    // connections still track (no missed local mutation => no silent loss).
    applying: Arc<AtomicBool>,
}

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
        Self::open_at(db_path())
    }

    pub fn open_at(path: PathBuf) -> Result<Self, String> {
        eprintln!("[db] opening {}", path.display());
        let conn =
            Connection::open(&path).map_err(|e| format!("DB open: {e}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("WAL: {e}"))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("FK: {e}"))?;
        conn.execute_batch("PRAGMA recursive_triggers=ON;")
            .map_err(|e| format!("recursive_triggers: {e}"))?;
        let applying = Arc::new(AtomicBool::new(false));
        {
            let flag = applying.clone();
            conn.create_scalar_function(
                "sync_is_applying",
                0,
                rusqlite::functions::FunctionFlags::SQLITE_UTF8,
                move |_ctx| Ok(i64::from(flag.load(Ordering::Relaxed))),
            )
            .map_err(|e| format!("register sync_is_applying: {e}"))?;
        }
        let db = Self {
            conn: Mutex::new(conn),
            applying,
        };
        db.migrate()?;
        db.init_sync()?;
        eprintln!("[db] ready");
        Ok(db)
    }

    // Toggle the connection-local apply flag. The sync service (RFC 0004, T17)
    // wraps a remote-batch application in set_applying(true)/false so the
    // tracking triggers no-op while it writes peer meta verbatim.
    pub fn set_applying(&self, applying: bool) {
        self.applying.store(applying, Ordering::Relaxed);
    }

    // Sync foundation (RFC 0004): ensure a stable device_id (read by the
    // tracking triggers) and seed sync_row_meta for pre-existing v1.0 rows.
    // Runs on every open; both steps are idempotent / once-guarded.
    fn init_sync(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let device_id = sync_meta::ensure_device_id(&conn)?;
        sync_meta::seed_sync_meta(&conn, &device_id)?;
        Ok(())
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
                if !sql.is_empty() {
                    conn.execute_batch(sql)
                        .map_err(|e| format!("Migration v{version}: {e}"))?;
                }
                if version == 4 {
                    self.migrate_audio_paths_to_relative(&conn);
                }
                if version == 10 {
                    sync_meta::install_sync_triggers(&conn)?;
                }
                conn.execute(
                    "INSERT OR IGNORE INTO _migrations (version) VALUES (?1)",
                    [version],
                )
                .map_err(|e| format!("Record v{version}: {e}"))?;
            }
        }
        Ok(())
    }

    fn migrate_audio_paths_to_relative(&self, conn: &Connection) {
        let mut stmt = match conn
            .prepare("SELECT id, audio_file_path FROM notes WHERE audio_file_path IS NOT NULL")
        {
            Ok(s) => s,
            Err(_) => return,
        };
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .ok()
            .map(|r| r.flatten().collect())
            .unwrap_or_default();
        for (id, path) in rows {
            if path.contains('/') {
                let filename = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path);
                let _ = conn.execute(
                    "UPDATE notes SET audio_file_path = ?1 WHERE id = ?2",
                    rusqlite::params![filename, id],
                );
                eprintln!("[db] v4 migrated audio path: {path} -> {filename}");
            }
        }
    }
}
