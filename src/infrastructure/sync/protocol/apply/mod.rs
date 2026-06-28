use super::catalog::{spec_for, KindSpec};
use super::collect::load_payload;
use super::wire::{SyncRow, SyncStats};
use super::{log, proto_err, sql_err};
use crate::infrastructure::persistence::sync_meta;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::conflict::{
    archive_conflict, decide, write_merged_meta, MergeOutcome, VersionInfo,
};
use crate::infrastructure::sync::vv::{parse_vv, vv_join};
use crate::infrastructure::sync::SyncError;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::BTreeMap;

mod archive;
mod entity;
mod hlc_guard;
mod meta;
mod reminders;
use archive::{archived_from_local, archived_from_payload};
use entity::{delete_entity, replace_chunks_from_payload, upsert_entity};
use hlc_guard::hlc_guard;
use meta::{load_local_meta, upsert_meta_verbatim, LocalMeta};
use reminders::{
    merge_reminder_intent, reactivate_twin_reminder, ReminderMerge,
};

const MAX_CHUNKS_PER_ROW: usize = 1024;

// Session facts the row-level merge needs. Snapshotted at HELLO time by
// session.rs and constant for the whole session.
// - peer_acked_of_mine: how far the peer has consumed MY seq space. A local
//   child row above it is an addition the peer has never seen -> concurrent
//   with any tombstone it pushes (add-wins-resurrect).
// - authority: I am the intact side of a full-state session (the peer was
//   restored or missed GC'd tombstones). I was never restored, so I hold a
//   meta row for everything I ever knew: an alive peer row I have NO meta
//   for is missing-locally = deleted, and is
//   archived before being re-deleted so it stays recoverable.
pub(super) struct ApplyCtx {
    pub full_state: bool,
    pub authority: bool,
    pub peer_acked_of_mine: i64,
    pub restored_session: bool,
    pub exempt_floor: Option<i64>,
    pub peer_device: String,
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
        let post_restore_creation = ctx
            .exempt_floor
            .map(|floor| {
                row.origin_device == ctx.peer_device && row.origin_seq > floor
            })
            .unwrap_or(false);
        if ctx.authority && row.deleted == 0 && post_restore_creation {
            log(&format!(
                "full-state: {}:{} authored above the restore floor -> \
                 genuine post-restore creation, exempted from re-delete",
                row.entity_kind, row.entity_id
            ));
        }
        if ctx.authority && row.deleted == 0 && !post_restore_creation {
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
    let outcome = if ctx.restored_session {
        hlc_guard(outcome, &local, row)
    } else {
        outcome
    };
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
        let origin_ok = row.origin_device == peer_device || ctx.full_state;
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
