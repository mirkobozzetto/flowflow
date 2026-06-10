use super::catalog::{entity_key_params, spec_for, KindSpec};
use super::collect::{load_chunks, load_payload};
use super::wire::{ChunkPayload, SyncRow, SyncStats};
use super::{log, proto_err, sql_err};
use crate::db::sync_meta;
use crate::db::Database;
use crate::services::sync::conflict::{
    archive_conflict, decide, write_merged_meta, ArchivedChunk, MergeOutcome,
    VersionInfo,
};
use crate::services::sync::vv::{parse_vv, vv_join};
use crate::services::sync::SyncError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::BTreeMap;

const MAX_CHUNKS_PER_ROW: usize = 1024;

// Session facts the row-level merge needs (T19). Snapshotted at HELLO time by
// session.rs and constant for the whole session.
// - peer_acked_of_mine: how far the peer has consumed MY seq space. A local
//   child row above it is an addition the peer has never seen -> concurrent
//   with any tombstone it pushes (add-wins-resurrect).
// - authority: I am the intact side of a full-state session (the peer was
//   restored or missed GC'd tombstones). I was never restored, so I hold a
//   meta row for everything I ever knew: an alive peer row I have NO meta
//   for is missing-locally = deleted (RFC 0004 reconciliation rule), and is
//   archived before being re-deleted so it stays recoverable.
pub(super) struct ApplyCtx {
    pub full_state: bool,
    pub authority: bool,
    pub peer_acked_of_mine: i64,
}

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

fn payload_intent(payload: Option<&Value>) -> Option<(String, String)> {
    let obj = payload?.as_object()?;
    Some((
        obj.get("note_id")?.as_str()?.to_string(),
        obj.get("intent_hash")?.as_str()?.to_string(),
    ))
}

fn find_twin_reminder(
    conn: &Connection,
    entity_id: &str,
    note_id: &str,
    intent_hash: &str,
) -> Result<Option<(String, String)>, SyncError> {
    conn.query_row(
        "SELECT id, state FROM note_reminders
         WHERE note_id = ?1 AND intent_hash = ?2 AND id != ?3",
        rusqlite::params![note_id, intent_hash, entity_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        e => Err(sql_err("reminder twin lookup", e)),
    })
}

// T23 cross-id cancel: a reminder tombstone whose id is unknown here may
// still target the SAME intent as a local row created independently (twin).
// A cancel anywhere kills the intent everywhere: flip the local twin to
// tombstone and author the change locally (the triggers are silenced during
// apply) so the cancel propagates back to every other device.
fn cancel_twin_reminder(
    conn: &Connection,
    entity_id: &str,
    payload: Option<&Value>,
) -> Result<(), SyncError> {
    let Some((note_id, intent_hash)) = payload_intent(payload) else {
        return Ok(());
    };
    let twin = find_twin_reminder(conn, entity_id, &note_id, &intent_hash)?;
    let Some((twin_id, state)) = twin else {
        return Ok(());
    };
    if state != "active" {
        return Ok(());
    }
    conn.execute(
        "UPDATE note_reminders SET state = 'tombstone' WHERE id = ?1",
        [&twin_id],
    )
    .map_err(|e| sql_err("cancel twin reminder", e))?;
    sync_meta::tombstone_entity(conn, "note_reminder", &twin_id)
        .map_err(SyncError::Protocol)?;
    log(&format!(
        "reminder cancel {entity_id} -> local twin {twin_id} (same intent)"
    ));
    Ok(())
}

// A version vector that strictly dominates the remote one: join + one local
// increment. Used wherever this device overrides a remote version with a
// locally-authored decision (full-state re-delete, twin cancel-wins), so the
// decision wins the merge on every peer instead of ping-ponging as a
// conflict.
fn dominating_vv(remote_vv: &str, my_device: &str) -> BTreeMap<String, i64> {
    let (mut vv, _) = parse_vv(remote_vv);
    *vv.entry(my_device.to_string()).or_insert(0) += 1;
    vv
}

fn meta_hlc(
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

// What to do with an incoming ALIVE reminder given the local twin landscape
// (T23, RFC MAJOR 10/16). The intent is the synced unit; the OS handle stays
// device-local, so a twin row keeps ITS id and ITS handle.
enum ReminderMerge {
    NoTwin,
    // An active local row already carries this intent: the remote row is the
    // same reminder under another id. Keep ours, skip theirs.
    KeepActiveTwin,
    // The local twin was cancelled, but the remote ALIVE version is NEWER
    // (the user re-created the intent after the cancel): reactivate ours.
    ReactivateTwin(String),
    // The local cancel is newer than the remote alive version: the cancel
    // wins; author a dominating tombstone for the remote id so the cancel
    // reaches the peer instead of its zombie reactivating ours forever.
    CancelRemote,
}

fn merge_reminder_intent(
    conn: &Connection,
    row: &SyncRow,
    payload: &Value,
) -> Result<ReminderMerge, SyncError> {
    let Some((note_id, intent_hash)) = payload_intent(Some(payload)) else {
        return Ok(ReminderMerge::NoTwin);
    };
    let twin =
        find_twin_reminder(conn, &row.entity_id, &note_id, &intent_hash)?;
    let Some((twin_id, state)) = twin else {
        return Ok(ReminderMerge::NoTwin);
    };
    if state == "active" {
        return Ok(ReminderMerge::KeepActiveTwin);
    }
    let cancel_hlc = meta_hlc(conn, "note_reminder", &twin_id)?;
    let remote_newer = match (&row.updated_hlc, &cancel_hlc) {
        (Some(remote), Some(local)) => remote > local,
        // Unknown ordering: bias toward the cancel (never resurrect a
        // notification the user explicitly killed).
        _ => false,
    };
    if remote_newer {
        Ok(ReminderMerge::ReactivateTwin(twin_id))
    } else {
        Ok(ReminderMerge::CancelRemote)
    }
}

fn reactivate_twin_reminder(
    conn: &Connection,
    twin_id: &str,
    payload: &Value,
) -> Result<(), SyncError> {
    let get = |k: &str| payload.get(k).cloned().unwrap_or(Value::Null);
    conn.execute(
        "UPDATE note_reminders SET
            state = 'active',
            due_year = ?2, due_month = ?3, due_day = ?4,
            due_hour = ?5, due_minute = ?6, is_all_day = ?7,
            tz_id = ?8, recurrence = ?9
         WHERE id = ?1",
        rusqlite::params![
            twin_id,
            json_to_sql(Some(&get("due_year"))),
            json_to_sql(Some(&get("due_month"))),
            json_to_sql(Some(&get("due_day"))),
            json_to_sql(Some(&get("due_hour"))),
            json_to_sql(Some(&get("due_minute"))),
            json_to_sql(Some(&get("is_all_day"))),
            json_to_sql(Some(&get("tz_id"))),
            json_to_sql(Some(&get("recurrence"))),
        ],
    )
    .map_err(|e| sql_err("reactivate twin reminder", e))?;
    sync_meta::mark_entity_updated(conn, "note_reminder", twin_id)
        .map_err(SyncError::Protocol)?;
    Ok(())
}

// How the row's CONTENT landed. The CALLER still records the row's meta for
// Applied/Skipped: every skip decision must be durable, because the watermark
// advances past this origin_seq and the row will never be resent.
// MetaWritten means the content path ALSO authored the row's meta itself
// (e.g. a dominating tombstone for a cancelled twin) - the caller must not
// overwrite it.
enum ContentOutcome {
    Applied,
    Skipped,
    MetaWritten,
}

fn apply_row_content(
    conn: &Connection,
    my_device: &str,
    spec: &KindSpec,
    row: &SyncRow,
) -> Result<ContentOutcome, SyncError> {
    if row.deleted != 0 {
        delete_entity(conn, spec, &row.entity_id, row.payload.as_ref())?;
        return Ok(ContentOutcome::Applied);
    }
    let Some(payload) = &row.payload else {
        log(&format!(
            "content skip {}:{} (live row without payload; a re-keyed \
             version or tombstone follows from the origin)",
            row.entity_kind, row.entity_id
        ));
        return Ok(ContentOutcome::Skipped);
    };
    if spec.kind == "note_reminder" {
        match merge_reminder_intent(conn, row, payload)? {
            ReminderMerge::NoTwin => {}
            ReminderMerge::KeepActiveTwin => {
                log(&format!(
                    "reminder {} merged into its local twin (same intent)",
                    row.entity_id
                ));
                return Ok(ContentOutcome::Skipped);
            }
            ReminderMerge::ReactivateTwin(twin_id) => {
                reactivate_twin_reminder(conn, &twin_id, payload)?;
                log(&format!(
                    "reminder {} re-added remotely -> twin {twin_id} \
                     reactivated",
                    row.entity_id
                ));
                return Ok(ContentOutcome::Applied);
            }
            ReminderMerge::CancelRemote => {
                let vv = dominating_vv(&row.version_vector, my_device);
                write_merged_meta(
                    conn,
                    my_device,
                    &row.entity_kind,
                    &row.entity_id,
                    &vv,
                    1,
                    &row.updated_hlc,
                )?;
                log(&format!(
                    "reminder {} arrived alive but the local cancel is \
                     newer -> cancel pushed back",
                    row.entity_id
                ));
                return Ok(ContentOutcome::MetaWritten);
            }
        }
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
    Ok(ContentOutcome::Applied)
}

// T19 add-wins-over-delete: children of `entity_id` that are alive locally,
// authored HERE, and never acked by the peer - additions the deleting device
// could not have known about. Their existence vetoes an incoming parent
// tombstone (the parent is resurrected instead, so no child is ever orphaned
// or silently destroyed).
fn concurrent_child_adds(
    conn: &Connection,
    my_device: &str,
    peer_acked_of_mine: i64,
    parent_kind: &str,
    entity_id: &str,
) -> Result<Vec<String>, SyncError> {
    let queries: &[(&str, &str)] = match parent_kind {
        "note" => &[
            (
                "attachment",
                "SELECT id FROM attachments WHERE note_id = ?1",
            ),
            (
                "note_audio",
                "SELECT id FROM note_audios WHERE note_id = ?1",
            ),
            (
                "note_reminder",
                "SELECT id FROM note_reminders
                 WHERE note_id = ?1 AND state = 'active'",
            ),
            (
                "notes_folders",
                "SELECT folder_id || ':' || note_id FROM notes_folders
                 WHERE note_id = ?1",
            ),
        ],
        "folder" => &[(
            "notes_folders",
            "SELECT folder_id || ':' || note_id FROM notes_folders
             WHERE folder_id = ?1",
        )],
        "conversation" => &[(
            "conversation_message",
            "SELECT id FROM conversation_messages
             WHERE conversation_id = ?1",
        )],
        _ => return Ok(Vec::new()),
    };
    let mut adds = Vec::new();
    for (child_kind, sql) in queries {
        for child_id in sync_meta::collect_ids(conn, sql, entity_id) {
            let unseen: bool = conn
                .query_row(
                    "SELECT deleted = 0 AND origin_device = ?3
                            AND origin_seq > ?4
                     FROM sync_row_meta
                     WHERE entity_kind = ?1 AND entity_id = ?2",
                    rusqlite::params![
                        child_kind,
                        child_id,
                        my_device,
                        peer_acked_of_mine
                    ],
                    |r| r.get(0),
                )
                .or_else(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => Ok(false),
                    e => Err(sql_err("child meta", e)),
                })?;
            if unseen {
                adds.push(format!("{child_kind}:{child_id}"));
            }
        }
    }
    Ok(adds)
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
    ctx: &ApplyCtx,
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
        // T19 full-state authority: I was never restored, so I hold meta for
        // every row I ever knew - an ALIVE row with NO local meta is one I
        // deleted whose tombstone was GC'd, resurrected by the peer's
        // restored backup. Missing-locally = deleted (RFC reconciliation
        // rule): re-delete it with a dominating tombstone, but archive the
        // pushed content first - the user can bring it back from the
        // conflicts screen, so nothing is ever silently lost. The same rule
        // intentionally catches a row the restored peer created BETWEEN its
        // restore and this session (indistinguishable by construction): it
        // lands in the archive instead of resurrecting a deleted row.
        if ctx.authority && row.deleted == 0 {
            archive_conflict(
                conn,
                &row.entity_kind,
                &row.entity_id,
                &row.version_vector,
                &row.payload.clone().unwrap_or(Value::Null),
                &archived_from_payload(&row.chunks),
                &row.updated_hlc,
            )?;
            let vv = dominating_vv(&row.version_vector, my_device);
            write_merged_meta(
                conn,
                my_device,
                &row.entity_kind,
                &row.entity_id,
                &vv,
                1,
                &row.updated_hlc,
            )?;
            stats.conflicts += 1;
            log(&format!(
                "full-state: {}:{} came back alive but its tombstone was \
                 GC'd here -> archived + re-deleted",
                row.entity_kind, row.entity_id
            ));
            return Ok(());
        }
        let landed = apply_row_content(conn, my_device, spec, row)?;
        match landed {
            ContentOutcome::Applied => {
                upsert_meta_verbatim(conn, row)?;
                stats.applied += 1;
            }
            ContentOutcome::Skipped => {
                upsert_meta_verbatim(conn, row)?;
                stats.skipped += 1;
            }
            ContentOutcome::MetaWritten => {
                stats.applied += 1;
            }
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
    // T19 add-wins-over-delete: before letting a remote tombstone win (by
    // dominance OR by tie-break), check for locally-alive children the
    // deleting device has never seen. Such an addition is concurrent with
    // the delete and WINS: the parent is re-authored alive (vv join + local
    // bump dominates the tombstone), so the resurrection propagates back and
    // no child is ever orphaned.
    let add_wins = if row.deleted != 0 && local.deleted == 0 {
        concurrent_child_adds(
            conn,
            my_device,
            ctx.peer_acked_of_mine,
            &row.entity_kind,
            &row.entity_id,
        )?
    } else {
        Vec::new()
    };
    match outcome {
        MergeOutcome::AlreadyCurrent => {
            stats.skipped += 1;
        }
        MergeOutcome::TakeRemote => {
            if !add_wins.is_empty() {
                let (local_vv, _) = parse_vv(&local.version_vector);
                let (remote_vv, _) = parse_vv(&row.version_vector);
                let mut joined = vv_join(&local_vv, &remote_vv);
                *joined.entry(my_device.to_string()).or_insert(0) += 1;
                write_merged_meta(
                    conn,
                    my_device,
                    &row.entity_kind,
                    &row.entity_id,
                    &joined,
                    0,
                    &local.updated_hlc,
                )?;
                stats.applied += 1;
                log(&format!(
                    "add-wins: tombstone for {}:{} rejected, concurrent \
                     children [{}] -> parent resurrected",
                    row.entity_kind,
                    row.entity_id,
                    add_wins.join(", ")
                ));
                return Ok(());
            }
            let landed = apply_row_content(conn, my_device, spec, row)?;
            match landed {
                ContentOutcome::Applied => {
                    upsert_meta_verbatim(conn, row)?;
                    stats.applied += 1;
                }
                ContentOutcome::Skipped => {
                    upsert_meta_verbatim(conn, row)?;
                    stats.skipped += 1;
                }
                ContentOutcome::MetaWritten => {
                    stats.applied += 1;
                }
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
            // A concurrent tombstone never beats a concurrent child add: the
            // tie-break is overridden and the local (alive) version wins.
            let remote_wins = remote_wins && add_wins.is_empty();
            if row.deleted != 0 && !add_wins.is_empty() {
                log(&format!(
                    "add-wins: concurrent tombstone for {}:{} loses to \
                     children [{}]",
                    row.entity_kind,
                    row.entity_id,
                    add_wins.join(", ")
                ));
            }
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
                let landed = apply_row_content(conn, my_device, spec, row)?;
                match landed {
                    // The twin cancel-wins path authored its own dominating
                    // meta; the merged-meta write below would clobber it.
                    ContentOutcome::MetaWritten => {
                        stats.applied += 1;
                    }
                    outcome => {
                        write_merged_meta(
                            conn,
                            my_device,
                            &row.entity_kind,
                            &row.entity_id,
                            &joined,
                            row.deleted,
                            &row.updated_hlc,
                        )?;
                        if matches!(outcome, ContentOutcome::Applied) {
                            stats.applied += 1;
                        }
                    }
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
    ctx: &ApplyCtx,
    rows: &[SyncRow],
    stats: &mut SyncStats,
) -> Result<i64, SyncError> {
    let conn = db.conn();
    if rows.is_empty() {
        return current_watermark(&conn, peer_device);
    }
    for row in rows {
        // Incremental: a peer only ever pushes rows it authored. A row
        // claiming another origin would corrupt that origin's watermark
        // accounting (and a row claiming MY origin could shadow my own seqs).
        // Full-state (T19): the peer replays its WHOLE table, which contains
        // my own rows verbatim; those are legitimate (and how a restored
        // device gets its lost data back). Anything else stays refused.
        let origin_ok = row.origin_device == peer_device
            || (ctx.full_state && row.origin_device == my_device);
        if !origin_ok {
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
        apply_row(&conn, my_device, ctx, row, stats)?;
    }
    // Only the PEER's own seqs advance my watermark of its space: a
    // full-state batch also replays my rows, whose seqs live in MY space.
    let max_seq = rows
        .iter()
        .filter(|r| r.origin_device == peer_device)
        .map(|r| r.origin_seq)
        .max()
        .unwrap_or(0);
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
