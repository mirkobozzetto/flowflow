pub mod attachment_repo;
pub mod chunk_repo;
pub mod conflict_repo;
pub mod conversation_repo;
pub mod folder_repo;
pub mod installed_agent_repo;
pub mod installed_connector_repo;
pub mod note_link_repo;
pub mod note_reminder_repo;
pub mod note_repo;
pub mod peer_repo;
pub mod pending_transcription_repo;
mod schema;
pub use schema::{TableClass, TABLES};
pub mod settings_repo;
pub mod share_repo;
pub mod space_repo;
pub use space_repo::{DbTx, PublishPending};
pub mod sync_meta;
pub mod thread_repo;

use rusqlite::{Connection, Transaction, TransactionBehavior};
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
        let dir = crate::infrastructure::platform::ios::documents_dir();
        std::fs::create_dir_all(&dir).ok();
        dir.join("flowflow.db")
    }
    #[cfg(not(target_os = "ios"))]
    {
        let dir = desktop_data_dir();
        std::fs::create_dir_all(&dir).ok();
        migrate_legacy_temp_data(&std::env::temp_dir().join("flowflow"), &dir);
        dir.join("flowflow.db")
    }
}

// Desktop data directory (issue #20): a real Mac install must never keep the
// only copy of the user's notes in temp_dir, which the OS purges. macOS gets
// Application Support; other desktop targets keep the historical temp dir.
// FLOWFLOW_DATA_DIR is the seam that points tests and secondary dev
// instances away from the real store.
#[cfg(not(target_os = "ios"))]
pub fn desktop_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("FLOWFLOW_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library/Application Support/FlowFlow");
    }
    std::env::temp_dir().join("flowflow")
}

// One-time, NON-destructive move-in: copy the legacy desktop store
// (temp_dir/flowflow) into the durable directory. Copies, never deletes - the
// legacy dir stays untouched as a fallback for an older build. The vector
// index is deliberately NOT copied: it is a derived cache the boot reconcile
// rebuilds from the SQLite BLOBs without any API call.
//
// CRASH-SAFE (review BLOCKER fix). The boot guard keys on flowflow.db
// EXISTING in new_dir, so that file must only appear once the copy is
// complete AND consistent, or a half-migration would be mistaken for a done
// one and the app would open a DB missing its WAL = silent loss. Therefore:
// - audio .wav files (secondary, outside the DB) are copied first, best
//   effort;
// - the SQLite DB is copied LAST via `VACUUM INTO` (one checkpointed,
//   consistent single-file image with NO -wal to lose) into a staging name,
//   then atomically renamed into place. The rename is the commit.
// Any failure removes the staging file and leaves new_dir without
// flowflow.db, so the next boot retries from the intact legacy store.
#[cfg(not(target_os = "ios"))]
pub fn migrate_legacy_temp_data(
    legacy: &std::path::Path,
    new_dir: &std::path::Path,
) {
    let legacy_db = legacy.join("flowflow.db");
    if legacy == new_dir
        || new_dir.join("flowflow.db").exists()
        || !legacy_db.exists()
    {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(legacy) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("wav")
            {
                if let Err(e) =
                    std::fs::copy(&path, new_dir.join(entry.file_name()))
                {
                    eprintln!("[db] legacy wav copy skipped: {e}");
                }
            }
        }
    }
    let staging = new_dir.join("flowflow.db.migrating");
    let _ = std::fs::remove_file(&staging);
    let result = (|| -> Result<(), String> {
        let conn = Connection::open(&legacy_db)
            .map_err(|e| format!("open legacy: {e}"))?;
        let target = staging
            .to_str()
            .ok_or_else(|| "non-utf8 staging path".to_string())?;
        conn.execute("VACUUM INTO ?1", [target])
            .map_err(|e| format!("vacuum into: {e}"))?;
        std::fs::rename(&staging, new_dir.join("flowflow.db"))
            .map_err(|e| format!("commit rename: {e}"))?;
        Ok(())
    })();
    match result {
        Ok(()) => eprintln!(
            "[db] migrated legacy store: {} -> {}",
            legacy.display(),
            new_dir.display()
        ),
        Err(e) => {
            let _ = std::fs::remove_file(&staging);
            eprintln!("[db] legacy migration skipped (retries next boot): {e}");
        }
    }
}

pub fn current_schema_version() -> i64 {
    MIGRATIONS.iter().map(|(v, _)| *v).max().unwrap_or(0)
}

pub fn raw_db_path() -> PathBuf {
    #[cfg(target_os = "ios")]
    {
        crate::infrastructure::platform::ios::documents_dir()
            .join("flowflow.db")
    }
    #[cfg(not(target_os = "ios"))]
    {
        desktop_data_dir().join("flowflow.db")
    }
}

pub fn now_iso() -> String {
    iso_at(chrono::Utc::now())
}

/// The one timestamp format every row and every comparison uses.
pub fn iso_at(at: chrono::DateTime<chrono::Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
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
        conn.execute_batch("PRAGMA busy_timeout=5000;")
            .map_err(|e| format!("busy_timeout: {e}"))?;
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
        #[cfg(target_os = "ios")]
        crate::infrastructure::platform::ios::sync_ffi::protect_db_files(&path);
        eprintln!("[db] ready");
        Ok(db)
    }

    // Move every WAL page back into the main DB file and truncate the WAL.
    // Called when the app heads to the background (RFC 0004 T16) so a lock or
    // eviction never catches data that only lives in the -wal file. The
    // pragma reports (busy, log, checkpointed): busy=1 means a reader blocked
    // the checkpoint, which must surface as an error, not a silent success.
    pub fn checkpoint_wal(&self) -> Result<(), String> {
        let conn = self.conn();
        let busy: i64 = conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
            .map_err(|e| format!("wal checkpoint: {e}"))?;
        if busy != 0 {
            return Err("wal checkpoint blocked by a reader".to_string());
        }
        Ok(())
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
        apply_migrations_with_effects(&conn, MIGRATIONS, |tx, version| {
            self.run_migration_side_effects(tx, version)
        })
    }

    fn run_migration_side_effects(
        &self,
        conn: &Connection,
        version: i64,
    ) -> Result<(), String> {
        if version == 4 {
            migrate_audio_paths_to_relative(conn);
        }
        if version == 10 || version == 13 || version == 25 {
            sync_meta::install_sync_triggers(conn)?;
        }
        if version == 17 {
            installed_connector_repo::backfill_sheets_pin(conn)?;
        }
        if version == 18 || version == 22 {
            sync_meta::install_sync_triggers(conn)?;
        }
        if version == 23 {
            let was_applying = self.applying.swap(true, Ordering::Relaxed);
            let result = backfill_author_device(conn);
            self.applying.store(was_applying, Ordering::Relaxed);
            result?;
        }
        Ok(())
    }
}

pub fn apply_migrations(
    conn: &Connection,
    migrations: &[(i64, &str)],
) -> Result<(), String> {
    apply_migrations_with_effects(conn, migrations, |_, _| Ok(()))
}

const REBUILDS: &[(i64, &str, &str)] = &[
    (11, "pending_transcriptions_v11", "pending_transcriptions"),
    (18, "conversation_messages_new", "conversation_messages"),
    (22, "conversation_messages_new", "conversation_messages"),
];

fn repair_interrupted_rebuild(
    tx: &Transaction<'_>,
    version: i64,
) -> Result<(), String> {
    let Some(&(_, temporary, original)) = REBUILDS
        .iter()
        .find(|&&(rebuild_version, _, _)| rebuild_version == version)
    else {
        return Ok(());
    };

    let table_exists = |name| {
        tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [name],
            |row| row.get::<_, bool>(0),
        )
    };
    let temporary_exists = table_exists(temporary)
        .map_err(|e| format!("Check temporary table for v{version}: {e}"))?;
    if !temporary_exists {
        return Ok(());
    }
    let original_exists = table_exists(original)
        .map_err(|e| format!("Check original table for v{version}: {e}"))?;
    let sql = if original_exists {
        format!("DROP TABLE {temporary}")
    } else {
        format!("ALTER TABLE {temporary} RENAME TO {original}")
    };
    tx.execute_batch(&sql)
        .map_err(|e| format!("Repair interrupted v{version}: {e}"))
}

fn apply_migrations_with_effects<F>(
    conn: &Connection,
    migrations: &[(i64, &str)],
    mut run_side_effects: F,
) -> Result<(), String>
where
    F: FnMut(&Transaction<'_>, i64) -> Result<(), String>,
{
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

    for &(version, sql) in migrations {
        if version <= current {
            continue;
        }
        eprintln!("[db] applying migration v{version}");
        let tx =
            Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
                .map_err(|e| format!("Begin v{version}: {e}"))?;
        repair_interrupted_rebuild(&tx, version)?;
        if !sql.is_empty() {
            tx.execute_batch(sql)
                .map_err(|e| format!("Migration v{version}: {e}"))?;
        }
        run_side_effects(&tx, version)?;
        tx.execute(
            "INSERT OR IGNORE INTO _migrations (version) VALUES (?1)",
            [version],
        )
        .map_err(|e| format!("Record v{version}: {e}"))?;
        tx.commit().map_err(|e| format!("Commit v{version}: {e}"))?;
    }
    Ok(())
}

fn migrate_audio_paths_to_relative(conn: &Connection) {
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

// V23 backfill: credit each pre-existing note to sync_row_meta.origin_device
// (last-writer, one-time approximation), then local-only rows to the local
// device id. Caller MUST hold the apply guard (set_applying) so the notes
// AFTER UPDATE trigger stays silent.
fn backfill_author_device(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "UPDATE notes SET author_device = (
             SELECT m.origin_device FROM sync_row_meta m
             WHERE m.entity_kind = 'note' AND m.entity_id = notes.id
         );
         UPDATE notes SET author_device =
             (SELECT value FROM settings WHERE key = 'sync_device_id')
             WHERE author_device IS NULL;",
    )
    .map_err(|e| format!("v23 author backfill: {e}"))
}

#[cfg(all(test, not(target_os = "ios")))]
mod migration_tests {
    use super::*;
    use crate::domain::note::NewTextNote;

    fn seed_legacy(dir: &std::path::Path) {
        let db = Database::open_at(dir.join("flowflow.db")).expect("legacy db");
        db.create_text_note(&NewTextNote {
            title: Some("legacy".into()),
            content: "survived the move".into(),
            tags: vec![],
        })
        .expect("seed note");
        // Intentionally NOT checkpointed: the row lives in the -wal file,
        // exactly the torn case the crash-safe copy must capture.
    }

    #[test]
    fn migration_captures_wal_and_commits_atomically() {
        let legacy = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        seed_legacy(legacy.path());

        migrate_legacy_temp_data(legacy.path(), target.path());

        assert!(target.path().join("flowflow.db").exists());
        assert!(
            !target.path().join("flowflow.db.migrating").exists(),
            "staging file must not survive a successful migration"
        );
        let db = Database::open_at(target.path().join("flowflow.db")).unwrap();
        let notes = db.list_notes().unwrap();
        assert_eq!(notes.len(), 1, "WAL-only data must be captured");
        assert_eq!(notes[0].content, "survived the move");
    }

    #[test]
    fn migration_skips_when_target_already_initialized() {
        let legacy = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        seed_legacy(legacy.path());
        // A target that already has its own DB must never be overwritten.
        Database::open_at(target.path().join("flowflow.db")).unwrap();

        migrate_legacy_temp_data(legacy.path(), target.path());

        let db = Database::open_at(target.path().join("flowflow.db")).unwrap();
        assert_eq!(
            db.list_notes().unwrap().len(),
            0,
            "existing target store must be left intact"
        );
    }
}
