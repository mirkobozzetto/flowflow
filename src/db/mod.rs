pub mod folder_repo;
pub mod note_repo;
mod schema;

use rusqlite::Connection;
use schema::MIGRATIONS;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
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
