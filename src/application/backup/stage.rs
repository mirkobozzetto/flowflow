use std::path::Path;

use rusqlite::Connection;

use super::fs_util::{crc32_of_file, fsync_dir, fsync_file};
use super::{
    activate_restore_lock, assert_no_sidecars, pending_restore_dir,
    RestoreState, ValidatedImport, RESTORE_STATE_PATH,
};

pub fn stage_import(validated: &ValidatedImport) -> Result<(), String> {
    stage_import_at(validated, &pending_restore_dir())
}

pub fn stage_import_at(
    validated: &ValidatedImport,
    pending_dir: &Path,
) -> Result<(), String> {
    let pending = pending_dir.to_path_buf();
    if pending.exists() {
        return Err(
            "a restore is already pending: restart the app first".to_string()
        );
    }

    {
        let conn = Connection::open(&validated.staged_db)
            .map_err(|e| format!("stage markers open: {e}"))?;
        conn.pragma_update(None, "journal_mode", "MEMORY")
            .map_err(|e| format!("stage markers journal: {e}"))?;
        let device_id: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'sync_device_id'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("stage markers device_id: {e}"))?;
        let floor: i64 = conn
            .query_row(
                "SELECT COALESCE(
                    (SELECT next_seq FROM sync_seq WHERE device_id = ?1), 0)",
                [&device_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("stage markers floor: {e}"))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('sync_restored_pending', 'true')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )
        .map_err(|e| format!("stage markers pending: {e}"))?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('sync_restored_floor', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [floor.to_string()],
        )
        .map_err(|e| format!("stage markers floor write: {e}"))?;
        eprintln!("[backup] restore markers staged (floor {floor})");

        drop(conn);
        let state = RestoreState {
            staged_db_crc32: crc32_of_file(&validated.staged_db)?,
            floor,
        };
        let state_json = serde_json::to_string(&state)
            .map_err(|e| format!("restore state serialize: {e}"))?;
        let state_file = validated.staging_dir.join(RESTORE_STATE_PATH);
        std::fs::write(&state_file, state_json)
            .map_err(|e| format!("restore state write: {e}"))?;
        fsync_file(&state_file)?;
    }
    assert_no_sidecars(&validated.staged_db)?;

    fsync_file(&validated.staged_db)?;
    if let Some(db_parent) = validated.staged_db.parent() {
        fsync_dir(db_parent)?;
    }
    fsync_dir(&validated.staging_dir)?;

    std::fs::rename(&validated.staging_dir, &pending)
        .map_err(|e| format!("stage commit rename: {e}"))?;
    if let Some(parent) = pending.parent() {
        fsync_dir(parent)?;
    }
    activate_restore_lock();
    eprintln!("[backup] import staged at {}", pending.display());
    Ok(())
}
