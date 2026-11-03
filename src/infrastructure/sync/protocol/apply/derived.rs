// Kinds remade from other data (a clip's words from its audio, its chapters
// from its words): no version of them is worth the conflicts screen, since the
// losing one can be made again.
use super::super::catalog::KindSpec;
use super::super::wire::{SyncRow, SyncStats};
use super::super::{log, sql_err};
use super::meta::LocalMeta;
use super::{apply_row_content, dominating_vv};
use crate::infrastructure::sync::conflict::write_merged_meta;
use crate::infrastructure::sync::SyncError;
use rusqlite::Connection;
use std::collections::BTreeMap;

fn clip_exists(conn: &Connection, audio_id: &str) -> Result<bool, SyncError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM note_audios WHERE id = ?1)",
        [audio_id],
        |r| r.get(0),
    )
    .map_err(|e| sql_err("clip exists", e))
}

/// Full-state authority, a live row this side holds no meta for. For derived
/// kinds that is not a resurrection: words a peer tracked first (both sides
/// seed them at v7) are kept while their clip lives here. Without the clip,
/// a dominating tombstone goes back, nothing archived. Returns whether the
/// row was settled here; otherwise it is applied as new.
pub(super) fn redeleted_without_clip(
    conn: &Connection,
    my_device: &str,
    row: &SyncRow,
) -> Result<bool, SyncError> {
    if clip_exists(conn, &row.entity_id)? {
        return Ok(false);
    }
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
        "full-state: {}:{} has no clip here -> re-deleted",
        row.entity_kind, row.entity_id
    ));
    Ok(true)
}

/// A concurrent write: the shared tie-break picks the same version on both
/// sides, with no archive and no conflict counted.
#[allow(clippy::too_many_arguments)]
pub(super) fn settle_concurrent(
    conn: &Connection,
    my_device: &str,
    spec: &KindSpec,
    row: &SyncRow,
    local: &LocalMeta,
    remote_wins: bool,
    joined: &BTreeMap<String, i64>,
    stats: &mut SyncStats,
) -> Result<(), SyncError> {
    let (deleted, hlc) = if remote_wins {
        apply_row_content(conn, my_device, spec, row)?;
        stats.applied += 1;
        (row.deleted, &row.updated_hlc)
    } else {
        (local.deleted, &local.updated_hlc)
    };
    write_merged_meta(
        conn,
        my_device,
        &row.entity_kind,
        &row.entity_id,
        joined,
        deleted,
        hlc,
    )?;
    log(&format!(
        "derived {}:{} settled (remote wins: {remote_wins})",
        row.entity_kind, row.entity_id
    ));
    Ok(())
}
