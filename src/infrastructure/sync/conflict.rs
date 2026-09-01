// RFC 0004 T18: the merge policy. Owns every decision made when two devices
// disagree about one row: dominance vs concurrence (version vectors), the
// deterministic tie-break, the archive of the LOSING version (snapshot +
// vector BLOBs, zero data loss, zero API call), the local re-authoring of the
// merged row, and the user-facing resolution (restore / dismiss).
//
// The protocol apply path calls decide()/archive_conflict()/write_merged_meta
// INSIDE its batch transaction; the resolution helpers run on the Database
// like any repo call.

use super::vv::{parse_vv, vv_cmp, vv_join, Dominance};
use super::SyncError;
use crate::domain::{NewTextNote, Note};
use crate::infrastructure::persistence::chunk_repo::{
    blob_to_vector, ChunkRecord,
};
use crate::infrastructure::persistence::conflict_repo::SyncConflict;
use crate::infrastructure::persistence::Database;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

// One side of a merge comparison, as stored in sync_row_meta / carried by a
// pushed row. The HLC only breaks ties between CONCURRENT versions; dominance
// itself never consults a clock.
pub struct VersionInfo<'a> {
    pub version_vector: &'a str,
    pub updated_hlc: Option<&'a str>,
    pub origin_device: &'a str,
}

#[derive(Debug, PartialEq)]
pub enum MergeOutcome {
    // Local is equal or strictly newer: idempotent skip.
    AlreadyCurrent,
    // Remote strictly newer: apply verbatim, meta copied as-is.
    TakeRemote,
    // True concurrent edit (or unreadable vector, forced here so corruption
    // can never silently lose data). The winner is the same on both devices.
    // The per-side corrupt flags feed the log: a corrupt LOCAL vector means
    // local DB damage, a corrupt REMOTE one a broken peer - different
    // investigations.
    Concurrent {
        remote_wins: bool,
        joined: BTreeMap<String, i64>,
        corrupt_local: bool,
        corrupt_remote: bool,
    },
}

pub fn decide(local: &VersionInfo, remote: &VersionInfo) -> MergeOutcome {
    let (local_vv, corrupt_local) = parse_vv(local.version_vector);
    let (remote_vv, corrupt_remote) = parse_vv(remote.version_vector);
    let dominance = if corrupt_local || corrupt_remote {
        Dominance::Concurrent
    } else {
        vv_cmp(&local_vv, &remote_vv)
    };
    match dominance {
        Dominance::Equal | Dominance::Local => MergeOutcome::AlreadyCurrent,
        Dominance::Remote => MergeOutcome::TakeRemote,
        Dominance::Concurrent => MergeOutcome::Concurrent {
            remote_wins: remote_wins_tiebreak(local, remote),
            joined: vv_join(&local_vv, &remote_vv),
            corrupt_local,
            corrupt_remote,
        },
    }
}

// Deterministic winner between two concurrent versions: highest
// (updated_hlc, origin_device) tuple. Both devices evaluate the identical
// comparison, so they pick the same winner without coordination. The HLC is
// hybrid-logical (skewed wall clocks of +/- minutes do not break ordering
// guarantees; they only bias which side wins, which is acceptable because the
// loser is archived, never dropped).
fn remote_wins_tiebreak(local: &VersionInfo, remote: &VersionInfo) -> bool {
    let l = (local.updated_hlc.unwrap_or(""), local.origin_device);
    let r = (remote.updated_hlc.unwrap_or(""), remote.origin_device);
    r > l
}

// A losing version's vector chunks, archived alongside its field snapshot.
// Reuses the BLOB bytes already on hand (RFC BLOCKER 3 fix: no re-embedding,
// no id fork); restoring a conflict later costs zero API calls.
#[derive(Debug, Serialize, Deserialize)]
pub struct ArchivedChunk {
    pub chunk_index: i64,
    pub vector_b64: String,
    pub content_hash: Option<String>,
    pub chunk_text: Option<String>,
    pub title: Option<String>,
    pub tags: Option<String>,
    pub created_at: String,
}

fn sql_err(ctx: &str, e: rusqlite::Error) -> SyncError {
    SyncError::Protocol(format!("{ctx}: {e}"))
}

// Archive the losing version of a concurrent edit. Runs INSIDE the caller's
// apply transaction: the archive and the winning row land atomically, so a
// cut can never apply a winner while dropping the loser.
pub fn archive_conflict(
    conn: &Connection,
    kind: &str,
    entity_id: &str,
    losing_vv: &str,
    losing_snapshot: &Value,
    losing_chunks: &[ArchivedChunk],
    created_hlc: &Option<String>,
) -> Result<(), SyncError> {
    let vector_ref = if losing_chunks.is_empty() {
        None
    } else {
        Some(serde_json::to_string(losing_chunks).map_err(|e| {
            SyncError::Protocol(format!("encode losing chunks: {e}"))
        })?)
    };
    conn.execute(
        "INSERT INTO sync_conflicts
            (entity_kind, entity_id, losing_vv, losing_snapshot_json,
             losing_vector_ref, created_hlc, resolved)
         VALUES (?1,?2,?3,?4,?5,?6,0)",
        rusqlite::params![
            kind,
            entity_id,
            losing_vv,
            losing_snapshot.to_string(),
            vector_ref,
            created_hlc,
        ],
    )
    .map_err(|e| sql_err("archive conflict", e))?;
    Ok(())
}

// Re-author a merged row locally: vv = join, fresh local origin_seq. The
// joined version then travels back to the peer as a normal local change, so
// both sides converge on identical meta without spurious future conflicts.
// Runs inside the caller's apply transaction (the seq bump is 2 statements).
pub fn write_merged_meta(
    conn: &Connection,
    my_device: &str,
    kind: &str,
    entity_id: &str,
    joined_vv: &BTreeMap<String, i64>,
    deleted: i64,
    updated_hlc: &Option<String>,
) -> Result<(), SyncError> {
    conn.execute(
        "UPDATE sync_seq SET next_seq = next_seq + 1 WHERE device_id = ?1",
        [my_device],
    )
    .map_err(|e| sql_err("merge seq bump", e))?;
    let seq: i64 = conn
        .query_row(
            "SELECT next_seq FROM sync_seq WHERE device_id = ?1",
            [my_device],
            |r| r.get(0),
        )
        .map_err(|e| sql_err("merge seq read", e))?;
    let vv = serde_json::to_string(joined_vv)
        .map_err(|e| SyncError::Protocol(format!("encode joined vv: {e}")))?;
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
            kind,
            entity_id,
            vv,
            my_device,
            seq,
            deleted,
            updated_hlc
        ],
    )
    .map_err(|e| sql_err("merge meta upsert", e))?;
    Ok(())
}

// ---- Resolution (user-facing, called from the UI) ----

fn snapshot_str(snapshot: &Value, key: &str) -> Option<String> {
    snapshot
        .get(key)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

// Restore an archived NOTE conflict as a brand-new note: the losing text
// comes back as a first-class synced entity (the create path fires the
// tracking triggers, so the restored note propagates to every device), and
// its archived vectors are copied back under the new note's deterministic
// chunk ids - zero re-embedding. The restored note also inherits the
// structure of the LIVE surviving original (its folders + thread), so
// recovering a conflict no longer detaches the note from where it lives
// (RFC 0007); if the original was deleted meanwhile, the restored note stays
// flat (safe fallback).
//
// Ordering (review fixes): the conflict is CLAIMED first (resolved=0 -> 1,
// guarded), so a double-tap or a stale list cannot run the restore twice and
// duplicate notes; if the note creation then fails, the claim is rolled back
// and the conflict stays visible. Once the note exists, every later step
// degrades gracefully (a skipped chunk re-embeds on the next edit) - no
// failure path can leave both a created note AND an unresolved conflict.
// After the chunks land, the owner's meta is bumped (chunks have no
// triggers; without the bump the vectors would never reach peers) and the
// engine is poked so the restored note propagates without waiting for an
// unrelated trigger.
pub fn restore_note_conflict(
    db: &Database,
    conflict: &SyncConflict,
) -> Result<Note, String> {
    if conflict.entity_kind != "note" {
        return Err(format!(
            "only note conflicts can be restored as a note (got {})",
            conflict.entity_kind
        ));
    }
    let snapshot: Value = serde_json::from_str(&conflict.losing_snapshot_json)
        .map_err(|e| format!("conflict snapshot unreadable: {e}"))?;
    let content = snapshot_str(&snapshot, "content").unwrap_or_default();
    let tags: Vec<String> = snapshot_str(&snapshot, "tags")
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    // Inherit structure from the LIVE surviving note (the winner at entity_id),
    // not the losing snapshot: folder links live only in the notes_folders
    // junction (absent from the snapshot), and "restore next to the original"
    // means where the note lives now. Read before the claim; both collapse to a
    // flat note if the original was deleted meanwhile.
    let folders = db.folders_for_note(&conflict.entity_id).unwrap_or_default();
    let live_thread = db
        .get_note(&conflict.entity_id)
        .ok()
        .flatten()
        .and_then(|n| n.thread_id)
        .filter(|tid| matches!(db.get_thread(tid), Ok(Some(_))));
    db.resolve_conflict(conflict.id)?;
    let new = NewTextNote {
        title: snapshot_str(&snapshot, "title"),
        content,
        tags,
    };
    // thread_id rides in the INSERT (atomic with the row); a fresh id means no
    // id fork, no new conflict.
    let create = match &live_thread {
        Some(tid) => db.create_text_note_in_thread(&new, tid),
        None => db.create_text_note(&new),
    };
    let note = match create {
        Ok(n) => n,
        Err(e) => {
            if let Err(rb) = db.unresolve_conflict(conflict.id) {
                eprintln!("[sync] conflict {} unclaim: {rb}", conflict.id);
            }
            return Err(e);
        }
    };
    // Folders are the only post-insert structural step. A failure here would
    // leave a detached note behind a resolved conflict (lost structure, no
    // retry), so roll the whole restore back and keep the conflict visible.
    for folder in &folders {
        if let Err(e) = db.add_note_to_folder(&note.id, &folder.id) {
            let _ = db.delete_note(&note.id);
            if let Err(rb) = db.unresolve_conflict(conflict.id) {
                eprintln!(
                    "[sync] conflict {} unclaim after folder re-link: {rb}",
                    conflict.id
                );
            }
            return Err(format!("restore folder re-link: {e}"));
        }
    }
    if let Some(raw) = &conflict.losing_vector_ref {
        match serde_json::from_str::<Vec<ArchivedChunk>>(raw) {
            Ok(chunks) => restore_chunks(db, &note.id, &chunks),
            Err(e) => eprintln!(
                "[sync] conflict {} vector ref unreadable ({e}); the \
                 restored note will re-embed on next edit",
                conflict.id
            ),
        }
    }
    crate::infrastructure::sync::engine::poke_after_data_change();
    Ok(note)
}

// Best-effort by design: a chunk that cannot come back (bad base64, wrong
// dimension, SQL failure) is logged and skipped - the note text is already
// safe and its next edit re-embeds everything. Erroring out here would leave
// a created note with a half-done restore behind a button the user retries.
fn restore_chunks(db: &Database, note_id: &str, chunks: &[ArchivedChunk]) {
    let mut records = Vec::with_capacity(chunks.len());
    for c in chunks {
        let blob = match URL_SAFE_NO_PAD.decode(&c.vector_b64) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "[sync] skip archived chunk idx {} (bad base64: {e})",
                    c.chunk_index
                );
                continue;
            }
        };
        if blob.len() != crate::application::constants::EMBEDDING_DIMS * 4 {
            eprintln!(
                "[sync] skip malformed archived chunk idx {} ({} bytes)",
                c.chunk_index,
                blob.len()
            );
            continue;
        }
        records.push(ChunkRecord {
            id: format!("note:{note_id}:{}", c.chunk_index),
            owner_id: note_id.to_string(),
            owner_kind: "note".to_string(),
            chunk_index: c.chunk_index as i32,
            embed_profile: crate::application::constants::EMBEDDING_PROFILE
                .to_string(),
            vector: blob_to_vector(&blob),
            content_hash: c.content_hash.clone().unwrap_or_default(),
            chunk_text: c.chunk_text.clone().unwrap_or_default(),
            title: c.title.clone().unwrap_or_default(),
            tags: c.tags.clone().unwrap_or_default(),
            created_at: c.created_at.clone(),
        });
    }
    if records.is_empty() {
        return;
    }
    match db.replace_chunks(note_id, "note", &records) {
        // Chunks have no tracking triggers: bump the owner meta (alive-check
        // + seq alloc in one IMMEDIATE tx, same invariant as the embed path)
        // or the restored vectors would never be collected for peers.
        Ok(()) => crate::application::embed::bump_owner_meta_if_alive(
            db, note_id, "note",
        ),
        Err(e) => eprintln!(
            "[sync] restored chunks for {note_id} not persisted ({e}); \
             the note will re-embed on next edit"
        ),
    }
}

pub fn dismiss_conflict(db: &Database, conflict_id: i64) -> Result<(), String> {
    db.resolve_conflict(conflict_id)
}
