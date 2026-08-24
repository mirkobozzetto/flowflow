use super::super::catalog::{entity_key_params, KindSpec};
use super::super::wire::ChunkPayload;
use super::super::{log, proto_err, sql_err};
use super::reminders::cancel_twin_reminder;
use crate::infrastructure::sync::SyncError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rusqlite::Connection;
use serde_json::Value;

pub(super) fn delete_entity(
    conn: &Connection,
    spec: &KindSpec,
    entity_id: &str,
    payload: Option<&Value>,
) -> Result<(), SyncError> {
    // Reminders: state is authoritative (RFC MAJOR 16). While the parent note
    // lives, a remote tombstone flips state='tombstone' (keeps the row so the
    // local OS-handle cleanup can see it and the reminder stops firing); the
    // row is only physically removed when its note is gone too.
    if spec.kind == "note_reminder" {
        let flipped = conn
            .execute(
                "UPDATE note_reminders SET state = 'tombstone'
             WHERE id = ?1 AND EXISTS
                (SELECT 1 FROM notes WHERE notes.id = note_reminders.note_id)",
                [entity_id],
            )
            .map_err(|e| sql_err("tombstone reminder", e))?;
        let removed = conn
            .execute(
                "DELETE FROM note_reminders
             WHERE id = ?1 AND NOT EXISTS
                (SELECT 1 FROM notes WHERE notes.id = note_reminders.note_id)",
                [entity_id],
            )
            .map_err(|e| sql_err("delete orphan reminder", e))?;
        if flipped + removed == 0 {
            cancel_twin_reminder(conn, entity_id, payload)?;
        }
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
        // The SQLite chunks are only half of it: the vectors live in LanceDB,
        // which this connection cannot reach from inside a sync transaction.
        // Queue the intent instead - the drain at boot and after each pull
        // finishes it. Without this a note deleted on one device and echoed
        // here in P2P keeps answering in chat on this one.
        conn.execute(
            "INSERT OR IGNORE INTO pending_purge (note_id, kind)
             VALUES (?1, ?2)",
            rusqlite::params![entity_id, spec.kind],
        )
        .map_err(|e| sql_err("queue entity purge", e))?;
    }
    Ok(())
}

pub(super) fn json_to_sql(v: Option<&Value>) -> rusqlite::types::Value {
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

pub(super) fn upsert_entity(
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

pub(super) fn replace_chunks_from_payload(
    conn: &Connection,
    owner_id: &str,
    owner_kind: &str,
    chunks: &[ChunkPayload],
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
        if blob.len() != crate::application::constants::EMBEDDING_DIMS * 4 {
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
