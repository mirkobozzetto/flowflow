// RFC 0004 T19: tombstone garbage collection, by acknowledgement.
//
// A tombstone row in sync_row_meta is the only thing preventing a deleted
// entity from being resurrected by a peer that still holds it alive. It can
// only be purged once EVERY paired peer provably consumed it:
// - my own tombstones (origin_device = me): purged when every peer's
//   persisted ack of MY seq space covers their origin_seq. The highest seq
//   ever purged becomes the gc_horizon advertised in HELLO - a peer whose
//   ack regresses below it (restored backup) forces a full-state session
//   instead of an incremental push that would miss the deletion.
// - a peer's tombstones (origin_device = peer): the peer authored them, so it
//   has them by definition; purged once my own watermark covers them. A
//   restored peer is caught by the same full-state detection.
//
// The peer's ack of MY space only exists on the wire (its HELLO + its ACKs
// while I push), so the session records it in a settings row per peer -
// same pattern as the PSK and the address book, wiped with the pairing.
// No peers paired = no GC: without a second device there is nothing to
// reconcile against, and tombstone rows are tiny.

use crate::db::sync_meta::DEVICE_ID_KEY;
use crate::db::Database;

pub const ACKED_BY_KEY_PREFIX: &str = "sync_peer_acked_by_";

fn acked_by_key(device_id: &str) -> String {
    format!("{ACKED_BY_KEY_PREFIX}{device_id}")
}

// Overwrite, never max: a restored peer legitimately REGRESSES its ack, and
// pretending otherwise would let the GC purge tombstones it never re-applied.
pub fn record_peer_ack(
    db: &Database,
    peer_device: &str,
    seq: i64,
) -> Result<(), String> {
    db.set_setting(&acked_by_key(peer_device), &seq.to_string())
}

pub fn peer_ack(db: &Database, peer_device: &str) -> i64 {
    db.get_setting(&acked_by_key(peer_device))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub fn clear_peer_ack(db: &Database, peer_device: &str) {
    let _ = db.set_setting(&acked_by_key(peer_device), "");
}

pub struct GcReport {
    pub purged: usize,
    pub horizon: i64,
}

pub fn run_gc(db: &Database) -> Result<GcReport, String> {
    let peers = db.list_peers()?;
    if peers.is_empty() {
        return Ok(GcReport {
            purged: 0,
            horizon: 0,
        });
    }
    let min_acked = peers
        .iter()
        .map(|p| peer_ack(db, &p.device_id))
        .min()
        .unwrap_or(0);
    let conn = db.conn();
    let my_device: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [DEVICE_ID_KEY],
            |r| r.get(0),
        )
        .map_err(|e| format!("gc device_id: {e}"))?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("gc tx: {e}"))?;
    // The horizon is the highest OWN seq actually purged, not min_acked:
    // a precise horizon only forces full-state when a deletion was really
    // lost to the incremental stream.
    let horizon: Option<i64> = tx
        .query_row(
            "SELECT MAX(origin_seq) FROM sync_row_meta
             WHERE deleted = 1 AND origin_device = ?1 AND origin_seq <= ?2",
            rusqlite::params![my_device, min_acked],
            |r| r.get(0),
        )
        .map_err(|e| format!("gc horizon scan: {e}"))?;
    let mut purged = tx
        .execute(
            "DELETE FROM sync_row_meta
             WHERE deleted = 1 AND origin_device = ?1 AND origin_seq <= ?2",
            rusqlite::params![my_device, min_acked],
        )
        .map_err(|e| format!("gc own tombstones: {e}"))?;
    for p in &peers {
        purged += tx
            .execute(
                "DELETE FROM sync_row_meta
                 WHERE deleted = 1 AND origin_device = ?1
                   AND origin_seq <= ?2",
                rusqlite::params![p.device_id, p.last_acked_seq],
            )
            .map_err(|e| format!("gc peer tombstones: {e}"))?;
    }
    if let Some(h) = horizon {
        tx.execute(
            "UPDATE sync_peers SET gc_horizon = MAX(gc_horizon, ?1)",
            [h],
        )
        .map_err(|e| format!("gc horizon update: {e}"))?;
    }
    tx.commit().map_err(|e| format!("gc commit: {e}"))?;
    let horizon = horizon.unwrap_or(0);
    if purged > 0 {
        eprintln!("[sync] gc: purged {purged} tombstones, horizon {horizon}");
    }
    Ok(GcReport { purged, horizon })
}
