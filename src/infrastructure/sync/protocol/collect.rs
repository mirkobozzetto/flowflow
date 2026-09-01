use super::catalog::{entity_key_params, spec_for, KindSpec};
use super::sql_err;
use super::wire::{ChunkPayload, SyncRow};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::SyncError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rusqlite::Connection;
use serde_json::Value;

// Read one entity row as a JSON payload, columns per its KindSpec. Also used
// by the apply side to snapshot the losing local version of a conflict.
pub(super) fn load_payload(
    conn: &Connection,
    spec: &KindSpec,
    entity_id: &str,
) -> Result<Option<Value>, SyncError> {
    let (where_clause, params) = entity_key_params(spec, entity_id)?;
    let sql = format!(
        "SELECT {} FROM {} WHERE {}",
        spec.cols.join(", "),
        spec.table,
        where_clause
    );
    let params: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
    let result = conn.query_row(&sql, params.as_slice(), |row| {
        let mut map = serde_json::Map::new();
        for (i, col) in spec.cols.iter().enumerate() {
            let v = match row.get_ref(i)? {
                rusqlite::types::ValueRef::Null => Value::Null,
                rusqlite::types::ValueRef::Integer(n) => Value::from(n),
                rusqlite::types::ValueRef::Real(f) => Value::from(f),
                rusqlite::types::ValueRef::Text(t) => {
                    Value::from(String::from_utf8_lossy(t).into_owned())
                }
                rusqlite::types::ValueRef::Blob(_) => Value::Null,
            };
            map.insert((*col).to_string(), v);
        }
        Ok(Value::Object(map))
    });
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(sql_err("load payload", e)),
    }
}

pub(super) fn load_chunks(
    conn: &Connection,
    owner_id: &str,
    owner_kind: &str,
) -> Result<Vec<ChunkPayload>, SyncError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, chunk_index, vector, content_hash, chunk_text,
                    title, tags, created_at, embed_profile
             FROM chunks WHERE owner_id = ?1 AND owner_kind = ?2
             ORDER BY chunk_index ASC",
        )
        .map_err(|e| sql_err("prepare chunks", e))?;
    let rows = stmt
        .query_map(rusqlite::params![owner_id, owner_kind], |row| {
            let blob: Vec<u8> = row.get(2)?;
            Ok(ChunkPayload {
                id: row.get(0)?,
                chunk_index: row.get(1)?,
                vector_b64: URL_SAFE_NO_PAD.encode(&blob),
                content_hash: row.get(3)?,
                chunk_text: row.get(4)?,
                title: row.get(5)?,
                tags: row.get(6)?,
                created_at: row.get(7)?,
                embed_profile: Some(row.get(8)?),
            })
        })
        .map_err(|e| sql_err("query chunks", e))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| sql_err("chunk row", e))?);
    }
    Ok(out)
}

// Collect the next batch of locally-authored rows above the peer's watermark.
// Meta + payload + chunks are read under one deferred transaction so each
// batch is a consistent snapshot.
pub(super) fn collect_batch(
    db: &Database,
    my_device: &str,
    after_seq: i64,
    limit: usize,
) -> Result<Vec<SyncRow>, SyncError> {
    let conn = db.conn();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| sql_err("collect tx", e))?;
    let mut metas: Vec<SyncRow> = Vec::new();
    {
        let mut stmt = tx
            .prepare(
                "SELECT entity_kind, entity_id, version_vector,
                        origin_device, origin_seq, deleted, updated_hlc
                 FROM sync_row_meta
                 WHERE origin_device = ?1 AND origin_seq > ?2
                 ORDER BY origin_seq ASC
                 LIMIT ?3",
            )
            .map_err(|e| sql_err("prepare collect", e))?;
        let rows = stmt
            .query_map(
                rusqlite::params![my_device, after_seq, limit as i64],
                |row| {
                    Ok(SyncRow {
                        entity_kind: row.get(0)?,
                        entity_id: row.get(1)?,
                        version_vector: row.get(2)?,
                        origin_device: row.get(3)?,
                        origin_seq: row.get(4)?,
                        deleted: row.get(5)?,
                        updated_hlc: row.get(6)?,
                        payload: None,
                        chunks: Vec::new(),
                    })
                },
            )
            .map_err(|e| sql_err("query collect", e))?;
        for r in rows {
            metas.push(r.map_err(|e| sql_err("collect row", e))?);
        }
    }
    fill_payloads(&tx, &mut metas)?;
    tx.commit().map_err(|e| sql_err("collect commit", e))?;
    Ok(metas)
}

fn fill_payloads(
    tx: &rusqlite::Transaction,
    metas: &mut [SyncRow],
) -> Result<(), SyncError> {
    for row in metas.iter_mut() {
        let Some(spec) = spec_for(&row.entity_kind) else {
            continue;
        };
        if row.deleted == 0 {
            row.payload = load_payload(tx, spec, &row.entity_id)?;
            if spec.chunk_owner {
                row.chunks = load_chunks(tx, &row.entity_id, spec.kind)?;
            }
        } else if spec.kind == "note_reminder" {
            // A reminder tombstone still has its soft-deleted row while the
            // note lives; ship its (note_id, intent_hash) so the receiver can
            // cancel its own TWIN row under a different id (T23 cross-id
            // cancel). None when the row is physically gone - the parent
            // note's tombstone kills the twin by cascade in that case.
            row.payload = load_payload(tx, spec, &row.entity_id)?;
        }
    }
    Ok(())
}

// Full-state collection (T19): EVERY meta row regardless of origin, paged by
// rowid. Used when one side was restored from a backup or missed GC'd
// tombstones: the incremental seq cursor cannot be trusted, so both sides
// exchange their entire state and let the row-level merge converge it.
// Returns the rows plus the last rowid for the next page.
pub(super) fn collect_full_batch(
    db: &Database,
    after_rowid: i64,
    limit: usize,
) -> Result<(Vec<SyncRow>, i64), SyncError> {
    let conn = db.conn();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| sql_err("collect full tx", e))?;
    let mut metas: Vec<SyncRow> = Vec::new();
    let mut last_rowid = after_rowid;
    {
        let mut stmt = tx
            .prepare(
                "SELECT rowid, entity_kind, entity_id, version_vector,
                        origin_device, origin_seq, deleted, updated_hlc
                 FROM sync_row_meta
                 WHERE rowid > ?1
                 ORDER BY rowid ASC
                 LIMIT ?2",
            )
            .map_err(|e| sql_err("prepare collect full", e))?;
        let rows = stmt
            .query_map(rusqlite::params![after_rowid, limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    SyncRow {
                        entity_kind: row.get(1)?,
                        entity_id: row.get(2)?,
                        version_vector: row.get(3)?,
                        origin_device: row.get(4)?,
                        origin_seq: row.get(5)?,
                        deleted: row.get(6)?,
                        updated_hlc: row.get(7)?,
                        payload: None,
                        chunks: Vec::new(),
                    },
                ))
            })
            .map_err(|e| sql_err("query collect full", e))?;
        for r in rows {
            let (rowid, meta) =
                r.map_err(|e| sql_err("collect full row", e))?;
            last_rowid = rowid;
            metas.push(meta);
        }
    }
    fill_payloads(&tx, &mut metas)?;
    tx.commit().map_err(|e| sql_err("collect full commit", e))?;
    Ok((metas, last_rowid))
}
