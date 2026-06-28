use super::super::wire::SyncRow;
use super::super::{log, sql_err};
use super::entity::json_to_sql;
use super::meta::meta_hlc;
use crate::infrastructure::persistence::sync_meta;
use crate::infrastructure::sync::SyncError;
use rusqlite::Connection;
use serde_json::Value;

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
pub(super) fn cancel_twin_reminder(
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

// What to do with an incoming ALIVE reminder given the local twin landscape
// (T23, RFC MAJOR 10/16). The intent is the synced unit; the OS handle stays
// device-local, so a twin row keeps ITS id and ITS handle.
pub(super) enum ReminderMerge {
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

pub(super) fn merge_reminder_intent(
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

pub(super) fn reactivate_twin_reminder(
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
