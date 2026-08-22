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
// same strict gate applies.
pub const PROTOCOL_VERSION: i64 = 4;

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
