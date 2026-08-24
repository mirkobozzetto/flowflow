mod apply;
mod catalog;
mod collect;
mod heal;
mod session;
mod wire;

pub use session::{serve_sync_once, sync_with_peer, ServeOutcome};
pub use wire::SyncStats;

pub const DEFAULT_BATCH_ROWS: usize = 100;
// v3: HELLO carries next_seq + restored; full-state sessions push every row
// by rowid and may author dominating tombstones. A v1/v2 peer half-running
// those semantics could resurrect deleted rows, so the version gate stays
// hard. v4: notes carry author_device in the catalog and HELLO exchanges
// device_name; an older peer would silently drop the author column, so the
// same strict gate applies. v5: notes and folders carry the space columns
// (proposal 0002), which a v4 peer has no schema for - its apply would fail on
// an unknown column, so the gate moves again.
pub const PROTOCOL_VERSION: i64 = 5;

/// The applier's entity delete, exposed for the test that guards the vector
/// purge it queues. Nothing in the app calls it directly: a real deletion
/// arrives inside a session, wrapped in meta and conflict handling.
pub fn apply_entity_delete_for_test(
    conn: &rusqlite::Connection,
    kind: &str,
    entity_id: &str,
) -> Result<(), SyncError> {
    let spec = catalog::spec_for(kind)
        .ok_or_else(|| proto_err(format!("unknown kind {kind}")))?;
    apply::delete_entity_for_test(conn, spec, entity_id)
}

// Catalog columns per entity kind, for the tests that guard what travels: a
// column absent from the fixed `cols` list never reaches another device.
pub fn synced_columns(
) -> std::collections::HashMap<&'static str, &'static [&'static str]> {
    catalog::KINDS.iter().map(|s| (s.kind, s.cols)).collect()
}

use super::SyncError;

pub(super) fn proto_err(msg: impl Into<String>) -> SyncError {
    SyncError::Protocol(msg.into())
}

pub(super) fn sql_err(ctx: &str, e: rusqlite::Error) -> SyncError {
    SyncError::Protocol(format!("{ctx}: {e}"))
}

pub(super) fn log(msg: &str) {
    eprintln!("[sync] {msg}");
}
