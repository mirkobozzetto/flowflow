use super::catalog::{entity_key_params, spec_for, KindSpec};
use super::collect::{load_chunks, load_payload};
use super::wire::{ChunkPayload, SyncRow, SyncStats};
use super::{log, proto_err, sql_err};
use crate::db::Database;
use crate::services::sync::conflict::{
    archive_conflict, decide, write_merged_meta, ArchivedChunk, MergeOutcome,
    VersionInfo,
};
use crate::services::sync::SyncError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rusqlite::Connection;
use serde_json::Value;

const MAX_CHUNKS_PER_ROW: usize = 1024;

struct LocalMeta {
    version_vector: String,
    origin_device: String,
    deleted: i64,
    updated_hlc: Option<String>,
}

fn load_local_meta(
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

fn upsert_meta_verbatim(
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

// The losing version's chunks, ready for the conflict archive. From the
// pushed payload when the REMOTE version loses; read back from the local
// chunks table (inside the apply tx, before the winner overwrites them) when
// the LOCAL version loses.
fn archived_from_payload(chunks: &[ChunkPayload]) -> Vec<ArchivedChunk> {
    chunks
        .iter()
        .map(|c| ArchivedChunk {
            chunk_index: c.chunk_index,
            vector_b64: c.vector_b64.clone(),
            content_hash: c.content_hash.clone(),
            chunk_text: c.chunk_text.clone(),
            title: c.title.clone(),
            tags: c.tags.clone(),
            created_at: c.created_at.clone(),
        })
        .collect()
}

fn archived_from_local(
    conn: &Connection,
    entity_id: &str,
    kind: &str,
) -> Result<Vec<ArchivedChunk>, SyncError> {
    Ok(archived_from_payload(&load_chunks(conn, entity_id, kind)?))
}

fn delete_entity(
    conn: &Connection,
    spec: &KindSpec,
    entity_id: &str,
) -> Result<(), SyncError> {
    // Reminders: state is authoritative (RFC MAJOR 16). While the parent note
    // lives, a remote tombstone flips state='tombstone' (keeps the row so the
    // local OS-handle cleanup can see it and the reminder stops firing); the
    // row is only physically removed when its note is gone too.
    if spec.kind == "note_reminder" {
        conn.execute(
            "UPDATE note_reminders SET state = 'tombstone'
             WHERE id = ?1 AND EXISTS
                (SELECT 1 FROM notes WHERE notes.id = note_reminders.note_id)",
            [entity_id],
        )
        .map_err(|e| sql_err("tombstone reminder", e))?;
        conn.execute(
            "DELETE FROM note_reminders
             WHERE id = ?1 AND NOT EXISTS
                (SELECT 1 FROM notes WHERE notes.id = note_reminders.note_id)",
            [entity_id],
        )
        .map_err(|e| sql_err("delete orphan reminder", e))?;
        return Ok(());
    }
    let (where_clause, params) = entity_key_params(spec, entity_id)?;
    let sql = format!("DELETE FROM {} WHERE {}", spec.table, where_clause);
    let params: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
    conn.execute(&sql, params.as_slice())
        .map_err(|e| sql_err("delete entity", e))?;
    if spec.chunk_owner {
        conn.execute(
            "DELETE FROM chunks WHERE owner_id = ?1 AND owner_kind = ?2",
            rusqlite::params![entity_id, spec.kind],
        )
        .map_err(|e| sql_err("delete entity chunks", e))?;
    }
    Ok(())
}

fn json_to_sql(v: Option<&Value>) -> rusqlite::types::Value {
    match v {
        None | Some(Value::Null) => rusqlite::types::Value::Null,
        Some(Value::Bool(b)) => rusqlite::types::Value::Integer(*b as i64),
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else {
                rusqlite::types::Value::Real(n.as_f64().unwrap_or(0.0))
            }
        }
        Some(Value::String(s)) => rusqlite::types::Value::Text(s.clone()),
        Some(other) => rusqlite::types::Value::Text(other.to_string()),
    }
}

fn upsert_entity(
    conn: &Connection,
    spec: &KindSpec,
    payload: &Value,
) -> Result<(), SyncError> {
    let obj = payload
        .as_object()
        .ok_or_else(|| proto_err("payload is not an object"))?;
    let placeholders: Vec<String> =
        (1..=spec.cols.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
        spec.table,
        spec.cols.join(", "),
        placeholders.join(", ")
    );
    let values: Vec<rusqlite::types::Value> =
        spec.cols.iter().map(|c| json_to_sql(obj.get(*c))).collect();
    let params: Vec<&dyn rusqlite::ToSql> =
        values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
    conn.execute(&sql, params.as_slice())
        .map_err(|e| sql_err("upsert entity", e))?;
    Ok(())
}

fn replace_chunks_from_payload(
    conn: &Connection,
    owner_id: &str,
    owner_kind: &str,
    chunks: &[super::wire::ChunkPayload],
) -> Result<(), SyncError> {
    conn.execute(
        "DELETE FROM chunks WHERE owner_id = ?1 AND owner_kind = ?2",
        rusqlite::params![owner_id, owner_kind],
    )
    .map_err(|e| sql_err("clear chunks", e))?;
    for c in chunks {
        let blob = URL_SAFE_NO_PAD
            .decode(&c.vector_b64)
            .map_err(|e| proto_err(format!("decode chunk blob: {e}")))?;
        // Same guard as reconcile: a malformed BLOB must never reach the
        // vector store (chunks_to_batch would panic on a bad dimension).
        if blob.len() != crate::services::constants::EMBEDDING_DIMS * 4 {
            log(&format!(
                "skip malformed chunk {} ({} bytes)",
                c.id,
                blob.len()
            ));
            continue;
        }
        conn.execute(
            "INSERT OR REPLACE INTO chunks
                (id, owner_id, owner_kind, chunk_index, dim, vector,
                 content_hash, chunk_text, title, tags, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            rusqlite::params![
                c.id,
                owner_id,
                owner_kind,
                c.chunk_index,
                (blob.len() / 4) as i64,
                blob,
                c.content_hash,
                c.chunk_text,
                c.title,
                c.tags,
                c.created_at,
            ],
        )
        .map_err(|e| sql_err("insert chunk", e))?;
    }
    Ok(())
}

// A remote reminder whose (note_id, intent_hash) already exists locally under
// a DIFFERENT id would either violate the UNIQUE constraint or, via OR
// REPLACE, silently swap the local row's identity. Same intent = same
// reminder: keep the local row, skip the remote one (T23 refines the merge).
fn reminder_intent_collides(
    conn: &Connection,
    payload: &Value,
    entity_id: &str,
) -> Result<bool, SyncError> {
    let obj = payload.as_object();
    let (Some(note_id), Some(intent_hash)) = (
        obj.and_then(|o| o.get("note_id")).and_then(Value::as_str),
        obj.and_then(|o| o.get("intent_hash"))
            .and_then(Value::as_str),
    ) else {
        return Ok(false);
    };
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM note_reminders
             WHERE note_id = ?1 AND intent_hash = ?2",
            rusqlite::params![note_id, intent_hash],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            e => Err(sql_err("reminder intent check", e)),
        })?;
    Ok(matches!(existing, Some(id) if id != entity_id))
}

// Write the row's CONTENT (entity table + chunks). Returns false when the
// content could not land (live row shipped without payload, or a reminder
// whose intent already exists locally under another id). The CALLER still
// records the row's meta: every skip decision must be durable, because the
// watermark advances past this origin_seq and the row will never be resent.
fn apply_row_content(
    conn: &Connection,
    spec: &KindSpec,
    row: &SyncRow,
) -> Result<bool, SyncError> {
    if row.deleted != 0 {
        delete_entity(conn, spec, &row.entity_id)?;
        return Ok(true);
    }
    let Some(payload) = &row.payload else {
        log(&format!(
            "content skip {}:{} (live row without payload; a re-keyed \
             version or tombstone follows from the origin)",
            row.entity_kind, row.entity_id
        ));
        return Ok(false);
    };
    if spec.kind == "note_reminder"
        && reminder_intent_collides(conn, payload, &row.entity_id)?
    {
        log(&format!(
            "content skip reminder {} (same intent exists locally, T23 \
             merges by intent)",
            row.entity_id
        ));
        return Ok(false);
    }
    upsert_entity(conn, spec, payload)?;
    if spec.chunk_owner {
        if row.chunks.len() > MAX_CHUNKS_PER_ROW {
            return Err(proto_err(format!(
                "row {} carries {} chunks (max {MAX_CHUNKS_PER_ROW})",
                row.entity_id,
                row.chunks.len()
            )));
        }
        replace_chunks_from_payload(
            conn,
            &row.entity_id,
            spec.kind,
            &row.chunks,
        )?;
    }
    Ok(true)
}

// Apply one row. INVARIANT (review BLOCKER fix): every outcome leaves a
// DURABLE record before the batch acks past this origin_seq. Either the meta
// is written (verbatim or merged), or the row provably needs nothing (local
// meta already Equal/dominating), or the whole batch errors (unknown kind =
// version skew: fail loudly, never consume). No transient-state skip may
// fall through silently, because a skipped seq is never resent.
fn apply_row(
    conn: &Connection,
    my_device: &str,
    row: &SyncRow,
    stats: &mut SyncStats,
) -> Result<(), SyncError> {
    let Some(spec) = spec_for(&row.entity_kind) else {
        return Err(proto_err(format!(
            "unknown entity kind '{}': the peer runs a newer protocol; \
             refusing the batch so nothing is consumed",
            row.entity_kind
        )));
    };
    let local = load_local_meta(conn, &row.entity_kind, &row.entity_id)?;
    let Some(local) = local else {
        let landed = apply_row_content(conn, spec, row)?;
        upsert_meta_verbatim(conn, row)?;
        if landed {
            stats.applied += 1;
        } else {
            stats.skipped += 1;
        }
        return Ok(());
    };
    let outcome = decide(
        &VersionInfo {
            version_vector: &local.version_vector,
            updated_hlc: local.updated_hlc.as_deref(),
            origin_device: &local.origin_device,
        },
        &VersionInfo {
            version_vector: &row.version_vector,
            updated_hlc: row.updated_hlc.as_deref(),
            origin_device: &row.origin_device,
        },
    );
    match outcome {
        MergeOutcome::AlreadyCurrent => {
            stats.skipped += 1;
        }
        MergeOutcome::TakeRemote => {
            let landed = apply_row_content(conn, spec, row)?;
            upsert_meta_verbatim(conn, row)?;
            if landed {
                stats.applied += 1;
            } else {
                stats.skipped += 1;
            }
        }
        MergeOutcome::Concurrent {
            remote_wins,
            joined,
            corrupt_local,
            corrupt_remote,
        } => {
            if corrupt_local || corrupt_remote {
                log(&format!(
                    "corrupt version vector on {}:{} (local: {corrupt_local}, \
                     remote: {corrupt_remote}) -> forced conflict",
                    row.entity_kind, row.entity_id
                ));
            }
            stats.conflicts += 1;
            if remote_wins {
                let losing_snapshot = load_payload(conn, spec, &row.entity_id)?
                    .unwrap_or(Value::Null);
                let losing_chunks = if spec.chunk_owner {
                    archived_from_local(conn, &row.entity_id, spec.kind)?
                } else {
                    Vec::new()
                };
                archive_conflict(
                    conn,
                    &row.entity_kind,
                    &row.entity_id,
                    &local.version_vector,
                    &losing_snapshot,
                    &losing_chunks,
                    &local.updated_hlc,
                )?;
                let landed = apply_row_content(conn, spec, row)?;
                write_merged_meta(
                    conn,
                    my_device,
                    &row.entity_kind,
                    &row.entity_id,
                    &joined,
                    row.deleted,
                    &row.updated_hlc,
                )?;
                if landed {
                    stats.applied += 1;
                }
            } else {
                let losing_snapshot =
                    row.payload.clone().unwrap_or(Value::Null);
                archive_conflict(
                    conn,
                    &row.entity_kind,
                    &row.entity_id,
                    &row.version_vector,
                    &losing_snapshot,
                    &archived_from_payload(&row.chunks),
                    &row.updated_hlc,
                )?;
                write_merged_meta(
                    conn,
                    my_device,
                    &row.entity_kind,
                    &row.entity_id,
                    &joined,
                    local.deleted,
                    &local.updated_hlc,
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn current_watermark(
    conn: &Connection,
    peer_device: &str,
) -> Result<i64, SyncError> {
    conn.query_row(
        "SELECT last_acked_seq FROM sync_peers WHERE device_id = ?1",
        [peer_device],
        |r| r.get(0),
    )
    .map_err(|e| sql_err("read watermark", e))
}

// Drop guard for the apply window: whatever the exit path (error, panic in a
// test harness), the open transaction is rolled back, the applying flag is
// lowered BEFORE the connection lock is released, and foreign_keys is
// restored. A lingering applying=true would make the NEXT local write
// untracked = silent loss; a lingering FK=OFF would skip cascades.
struct ApplyGuard<'a> {
    db: &'a Database,
    conn: &'a Connection,
    armed: bool,
}

impl Drop for ApplyGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.conn.execute_batch("ROLLBACK;");
        }
        self.db.set_applying(false);
        let _ = self.conn.execute_batch("PRAGMA foreign_keys=ON;");
    }
}

// Apply one PUSH batch atomically: rows + watermark advance in a single
// IMMEDIATE transaction (write lock taken upfront: a concurrent embed-thread
// commit cannot abort us with SQLITE_BUSY_SNAPSHOT mid-batch; it waits on
// busy_timeout instead). The connection-local applying flag silences the
// tracking triggers so peer meta is written verbatim. Foreign keys are
// disabled for the apply only: seq order does not follow FK order (an UPDATE
// re-keys its row past its children), and every child carries its own
// row/tombstone, so the end state is consistent.
pub(super) fn apply_batch(
    db: &Database,
    my_device: &str,
    peer_device: &str,
    rows: &[SyncRow],
    stats: &mut SyncStats,
) -> Result<i64, SyncError> {
    let conn = db.conn();
    if rows.is_empty() {
        return current_watermark(&conn, peer_device);
    }
    for row in rows {
        // v1 protocol: a peer only ever pushes rows it authored. A row
        // claiming another origin would corrupt that origin's watermark
        // accounting (and a row claiming MY origin could shadow my own seqs).
        if row.origin_device != peer_device {
            return Err(proto_err(format!(
                "row {}:{} claims origin '{}' but the authenticated peer \
                 is '{}'",
                row.entity_kind, row.entity_id, row.origin_device, peer_device
            )));
        }
    }
    conn.execute_batch("PRAGMA foreign_keys=OFF;")
        .map_err(|e| sql_err("disable foreign_keys", e))?;
    db.set_applying(true);
    let mut guard = ApplyGuard {
        db,
        conn: &conn,
        armed: false,
    };
    conn.execute_batch("BEGIN IMMEDIATE;")
        .map_err(|e| sql_err("apply tx", e))?;
    guard.armed = true;
    for row in rows {
        apply_row(&conn, my_device, row, stats)?;
    }
    let max_seq = rows.iter().map(|r| r.origin_seq).max().unwrap_or(0);
    conn.execute(
        "UPDATE sync_peers SET last_acked_seq = MAX(last_acked_seq, ?1)
         WHERE device_id = ?2",
        rusqlite::params![max_seq, peer_device],
    )
    .map_err(|e| sql_err("advance watermark", e))?;
    conn.execute_batch("COMMIT;")
        .map_err(|e| sql_err("apply commit", e))?;
    guard.armed = false;
    drop(guard);
    current_watermark(&conn, peer_device)
}
