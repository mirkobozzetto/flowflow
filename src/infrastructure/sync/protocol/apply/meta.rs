use super::super::sql_err;
use super::super::wire::SyncRow;
use crate::infrastructure::sync::SyncError;
use rusqlite::Connection;

pub(super) struct LocalMeta {
    pub(super) version_vector: String,
    pub(super) origin_device: String,
    pub(super) deleted: i64,
    pub(super) updated_hlc: Option<String>,
}

pub(super) fn load_local_meta(
    conn: &Connection,
    kind: &str,
    entity_id: &str,
) -> Result<Option<LocalMeta>, SyncError> {
    let result = conn.query_row(
        "SELECT version_vector, origin_device, deleted, updated_hlc
         FROM sync_row_meta WHERE entity_kind = ?1 AND entity_id = ?2",
        rusqlite::params![kind, entity_id],
        |row| {
            Ok(LocalMeta {
                version_vector: row.get(0)?,
                origin_device: row.get(1)?,
                deleted: row.get(2)?,
                updated_hlc: row.get(3)?,
            })
        },
    );
    match result {
        Ok(m) => Ok(Some(m)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(sql_err("load local meta", e)),
    }
}

pub(super) fn upsert_meta_verbatim(
    conn: &Connection,
    row: &SyncRow,
) -> Result<(), SyncError> {
    conn.execute(
        "INSERT INTO sync_row_meta
            (entity_kind, entity_id, version_vector, origin_device,
             origin_seq, deleted, updated_hlc)
         VALUES (?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(entity_kind, entity_id) DO UPDATE SET
            version_vector = excluded.version_vector,
            origin_device = excluded.origin_device,
            origin_seq = excluded.origin_seq,
            deleted = excluded.deleted,
            updated_hlc = excluded.updated_hlc",
        rusqlite::params![
            row.entity_kind,
            row.entity_id,
            row.version_vector,
            row.origin_device,
            row.origin_seq,
            row.deleted,
            row.updated_hlc,
        ],
    )
    .map_err(|e| sql_err("upsert meta", e))?;
    Ok(())
}

pub(super) fn meta_hlc(
    conn: &Connection,
    kind: &str,
    entity_id: &str,
) -> Result<Option<String>, SyncError> {
    conn.query_row(
        "SELECT updated_hlc FROM sync_row_meta
         WHERE entity_kind = ?1 AND entity_id = ?2",
        rusqlite::params![kind, entity_id],
        |r| r.get(0),
    )
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        e => Err(sql_err("meta hlc", e)),
    })
}
