use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::infrastructure::persistence::settings_repo::{
    DEVICE_LOCAL_SETTINGS, DEVICE_LOCAL_SETTING_PREFIXES, SENSITIVE_SETTINGS,
    SENSITIVE_SETTING_PREFIXES,
};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::reconcile;
use crate::infrastructure::vectordb::VectorStore;

use super::{assert_no_sidecars, EXCLUDED_TABLES};

pub async fn ensure_chunks_backfilled(
    db: &Database,
    store: &VectorStore,
) -> Result<(), String> {
    if reconcile::is_backfilled(db) {
        return Ok(());
    }
    eprintln!("[backup] chunk backfill not done, running it before export");
    reconcile::backfill_legacy_chunks(db, store).await
}

pub fn create_scrubbed_snapshot(
    source_db: &Path,
    staging_dir: &Path,
) -> Result<PathBuf, String> {
    if staging_dir.exists() {
        std::fs::remove_dir_all(staging_dir)
            .map_err(|e| format!("staging reset: {e}"))?;
    }
    std::fs::create_dir_all(staging_dir)
        .map_err(|e| format!("staging create: {e}"))?;
    let snapshot = staging_dir.join("flowflow.db");

    let source = Connection::open_with_flags(
        source_db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("snapshot source open: {e}"))?;
    source
        .execute_batch("PRAGMA busy_timeout=5000;")
        .map_err(|e| format!("snapshot busy_timeout: {e}"))?;
    let target = snapshot
        .to_str()
        .ok_or_else(|| "non-utf8 snapshot path".to_string())?;
    source
        .execute("VACUUM INTO ?1", [target])
        .map_err(|e| format!("vacuum into: {e}"))?;
    drop(source);

    scrub_snapshot(&snapshot)?;
    assert_no_sidecars(&snapshot)?;
    Ok(snapshot)
}

fn scrub_snapshot(snapshot: &Path) -> Result<(), String> {
    let conn =
        Connection::open(snapshot).map_err(|e| format!("scrub open: {e}"))?;
    conn.pragma_update(None, "journal_mode", "MEMORY")
        .map_err(|e| format!("scrub journal_mode: {e}"))?;
    conn.pragma_update(None, "secure_delete", "ON")
        .map_err(|e| format!("scrub secure_delete: {e}"))?;
    for key in SENSITIVE_SETTINGS
        .iter()
        .chain(DEVICE_LOCAL_SETTINGS.iter())
    {
        conn.execute("DELETE FROM settings WHERE key = ?1", [key])
            .map_err(|e| format!("scrub setting {key}: {e}"))?;
    }
    for prefix in SENSITIVE_SETTING_PREFIXES
        .iter()
        .chain(DEVICE_LOCAL_SETTING_PREFIXES.iter())
    {
        conn.execute(
            "DELETE FROM settings WHERE substr(key, 1, ?1) = ?2",
            rusqlite::params![prefix.len() as i64, prefix],
        )
        .map_err(|e| format!("scrub prefix {prefix}: {e}"))?;
    }
    for table in EXCLUDED_TABLES {
        conn.execute(&format!("DELETE FROM {table}"), [])
            .map_err(|e| format!("scrub table {table}: {e}"))?;
    }
    conn.execute_batch("VACUUM;")
        .map_err(|e| format!("scrub vacuum: {e}"))?;
    drop(conn);
    Ok(())
}
