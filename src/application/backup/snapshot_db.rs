use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use super::Counts;

pub fn open_read_only(db_file: &Path) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(
        db_file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("read-only open: {e}"))?;
    conn.execute_batch("PRAGMA busy_timeout=5000;")
        .map_err(|e| format!("read-only busy_timeout: {e}"))?;
    Ok(conn)
}

pub fn audio_paths_from_snapshot(
    snapshot: &Path,
) -> Result<Vec<String>, String> {
    let conn = open_read_only(snapshot)?;
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT file_path FROM note_audios ORDER BY file_path",
        )
        .map_err(|e| format!("audio paths prepare: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("audio paths query: {e}"))?;
    let mut paths = Vec::new();
    for row in rows {
        paths.push(row.map_err(|e| format!("audio paths row: {e}"))?);
    }
    Ok(paths)
}

pub(crate) fn snapshot_device_id(snapshot: &Path) -> Result<String, String> {
    let conn = open_read_only(snapshot)?;
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'sync_device_id'",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("snapshot device_id: {e}"))
}

pub fn snapshot_counts(db_file: &Path) -> Result<Counts, String> {
    let conn = open_read_only(db_file)?;
    let count = |table: &str| -> Result<i64, String> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .map_err(|e| format!("count {table}: {e}"))
    };
    Ok(Counts {
        notes: count("notes")?,
        folders: count("folders")?,
        threads: count("threads")?,
        attachments: count("attachments")?,
        conversations: count("conversations")?,
        audio_files: count("note_audios")?,
        chunks: count("chunks")?,
        reminders: count("note_reminders")?,
    })
}
