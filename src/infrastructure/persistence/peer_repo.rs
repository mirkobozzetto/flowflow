use crate::infrastructure::persistence::{now_iso, Database};

#[derive(Debug, Clone, PartialEq)]
pub struct SyncPeer {
    pub device_id: String,
    pub static_pubkey: String,
    pub last_acked_seq: i64,
    pub paired_at: Option<String>,
    pub gc_horizon: i64,
}

fn row_to_peer(row: &rusqlite::Row) -> rusqlite::Result<SyncPeer> {
    Ok(SyncPeer {
        device_id: row.get("device_id")?,
        static_pubkey: row.get("static_pubkey")?,
        last_acked_seq: row.get("last_acked_seq")?,
        paired_at: row.get("paired_at")?,
        gc_horizon: row.get("gc_horizon")?,
    })
}

impl Database {
    pub fn upsert_peer(
        &self,
        device_id: &str,
        static_pubkey: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT INTO sync_peers (device_id, static_pubkey, paired_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(device_id) DO UPDATE SET
                   static_pubkey = excluded.static_pubkey,
                   paired_at = excluded.paired_at",
                rusqlite::params![device_id, static_pubkey, now_iso()],
            )
            .map_err(|e| format!("upsert peer: {e}"))?;
        Ok(())
    }

    pub fn get_peer(
        &self,
        device_id: &str,
    ) -> Result<Option<SyncPeer>, String> {
        self.conn()
            .query_row(
                "SELECT device_id, static_pubkey, last_acked_seq, paired_at,
                        gc_horizon
                 FROM sync_peers WHERE device_id = ?1",
                [device_id],
                row_to_peer,
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(format!("get peer: {e}")),
            })
    }

    pub fn list_peers(&self) -> Result<Vec<SyncPeer>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT device_id, static_pubkey, last_acked_seq, paired_at,
                        gc_horizon
                 FROM sync_peers ORDER BY paired_at DESC",
            )
            .map_err(|e| format!("list peers: {e}"))?;
        let peers = stmt
            .query_map([], row_to_peer)
            .map_err(|e| format!("list peers: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list peers: {e}"))?;
        Ok(peers)
    }

    pub fn delete_peer(&self, device_id: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM sync_peers WHERE device_id = ?1", [device_id])
            .map_err(|e| format!("delete peer: {e}"))?;
        Ok(())
    }

    // Persist a pairing (peer row + per-peer PSK setting) atomically: a crash
    // mid-pairing never leaves a peer without its PSK (which would make every
    // future sync session unable to open a Noise channel to it).
    pub fn persist_pairing(
        &self,
        device_id: &str,
        static_pubkey: &str,
        psk_key: &str,
        psk_b64: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("persist pairing tx: {e}"))?;
        tx.execute(
            "INSERT INTO sync_peers (device_id, static_pubkey, paired_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(device_id) DO UPDATE SET
               static_pubkey = excluded.static_pubkey,
               paired_at = excluded.paired_at",
            rusqlite::params![device_id, static_pubkey, now_iso()],
        )
        .map_err(|e| format!("persist peer: {e}"))?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![psk_key, psk_b64],
        )
        .map_err(|e| format!("persist peer psk: {e}"))?;
        tx.commit()
            .map_err(|e| format!("persist pairing commit: {e}"))?;
        Ok(())
    }

    // Remove a pairing (peer row + PSK setting row) atomically. Deletes the PSK
    // row outright rather than blanking it, so no stale secret lingers.
    pub fn delete_pairing(
        &self,
        device_id: &str,
        psk_key: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("delete pairing tx: {e}"))?;
        tx.execute("DELETE FROM sync_peers WHERE device_id = ?1", [device_id])
            .map_err(|e| format!("delete peer: {e}"))?;
        tx.execute("DELETE FROM settings WHERE key = ?1", [psk_key])
            .map_err(|e| format!("delete peer psk: {e}"))?;
        tx.commit()
            .map_err(|e| format!("delete pairing commit: {e}"))?;
        Ok(())
    }
}
