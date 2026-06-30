mod apply;
mod catalog;
mod collect;
mod session;
mod wire;

pub use session::{serve_sync_once, sync_with_peer, ServeOutcome};
pub use wire::SyncStats;

pub const DEFAULT_BATCH_ROWS: usize = 100;
// v2: HELLO carries next_seq; full-state sessions push every row by
// rowid and may author dominating tombstones. A v1 peer half-running those
// semantics could resurrect deleted rows, so the version gate stays hard.
pub const PROTOCOL_VERSION: i64 = 3;

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
