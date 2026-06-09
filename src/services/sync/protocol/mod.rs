// RFC 0004 T17: HELLO/PUSH/ACK protocol over the Noise channel.
//
// Each device pushes ONLY the rows it authored (origin_device = me) with
// origin_seq above the peer's persisted watermark, in ascending seq order.
// The receiver applies each batch in ONE transaction that also advances its
// sync_peers.last_acked_seq watermark: a cut at any point leaves applied-and-
// acked or nothing, so a resumed session continues exactly where it stopped.
//
// Merge rule: owned by services::sync::conflict (T18). apply.rs asks
// conflict::decide() and executes the outcome:
// - no local meta, or remote version vector dominates -> apply verbatim
// - local dominates or equal -> skip (idempotent replay)
// - concurrent -> deterministic winner by (updated_hlc, origin_device), the
//   SAME on both devices; the losing version is ARCHIVED in sync_conflicts
//   (snapshot + vector BLOBs, never silently dropped: zero data loss), and
//   the merged row is RE-AUTHORED locally (vv = join, fresh local origin_seq)
//   so the join flows back to the other side within the same session.
//
// Vectors: a note/attachment row travels with its chunks (BLOB base64) so the
// second device never re-embeds (RFC variant B). Chunks have no triggers; the
// embed path bumps the owner's meta after persisting fresh BLOBs.
//
// Module layout (SRP):
// - wire:    message types + encrypted send/recv
// - catalog: the synced-entity registry (kind -> table/columns/key)
// - collect: sender side (enumerate meta + payload + chunks per batch)
// - apply:   receiver side (executes merge outcomes, atomic batch apply)
// - session: HELLO checks + full bidirectional sessions
// Version-vector algebra lives in sync::vv; merge policy in sync::conflict.

mod apply;
mod catalog;
mod collect;
mod session;
mod wire;

pub use session::{serve_sync_once, sync_with_peer, ServeOutcome};
pub use wire::SyncStats;

pub const DEFAULT_BATCH_ROWS: usize = 100;
pub const PROTOCOL_VERSION: i64 = 1;

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
