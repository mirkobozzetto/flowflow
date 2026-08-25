// Persistence for the device-side share records (proposal 0001, V25):
// note_shares (my live publications) and note_provenance (kept-content
// origins). Both are sync-tracked by triggers; deletes go through
// tombstone_entity like every synced table.

use crate::domain::share::{LocalShare, Provenance, ShareKind};
use crate::infrastructure::persistence::{now_iso, sync_meta, Database, DbTx};

impl Database {
    // Record (or replace) my live share for a source. One row per source:
    // republish overwrites code + expiry.
    pub fn upsert_share(
        &self,
        source_id: &str,
        kind: &ShareKind,
        code: &str,
        expires_at: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO note_shares (id, kind, code, expires_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind, code = excluded.code,
                expires_at = excluded.expires_at,
                modified_at = excluded.modified_at",
            rusqlite::params![
                source_id,
                kind.as_str(),
                code,
                expires_at,
                now_iso()
            ],
        )
        .map_err(|e| format!("upsert share: {e}"))?;
        Ok(())
    }

    pub fn upsert_provenance(&self, p: &Provenance) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO note_provenance
                (id, share_code, remote_note_id, author_name, state, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                share_code = excluded.share_code,
                remote_note_id = excluded.remote_note_id,
                author_name = excluded.author_name,
                state = excluded.state,
                modified_at = excluded.modified_at",
            rusqlite::params![
                p.note_id,
                p.share_code,
                p.remote_note_id,
                p.author_name,
                p.state,
                now_iso()
            ],
        )
        .map_err(|e| format!("upsert provenance: {e}"))?;
        Ok(())
    }

    pub fn get_provenance(&self, note_id: &str) -> Option<Provenance> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, share_code, remote_note_id, author_name, captured_at, state
             FROM note_provenance WHERE id = ?1",
            [note_id],
            |row| {
                Ok(Provenance {
                    note_id: row.get(0)?,
                    share_code: row.get(1)?,
                    remote_note_id: row.get(2)?,
                    author_name: row.get(3)?,
                    captured_at: row.get(4)?,
                    state: row.get(5)?,
                })
            },
        )
        .ok()
    }

    // Every kept note that came from one share (the alignment check walks these).
    pub fn provenances_for_code(&self, share_code: &str) -> Vec<Provenance> {
        let conn = self.conn();
        let mut stmt = match conn.prepare(
            "SELECT id, share_code, remote_note_id, author_name, captured_at, state
             FROM note_provenance WHERE share_code = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([share_code], |row| {
            Ok(Provenance {
                note_id: row.get(0)?,
                share_code: row.get(1)?,
                remote_note_id: row.get(2)?,
                author_name: row.get(3)?,
                captured_at: row.get(4)?,
                state: row.get(5)?,
            })
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    pub fn all_provenance_codes(&self) -> Vec<String> {
        let conn = self.conn();
        let mut stmt = match conn
            .prepare("SELECT DISTINCT share_code FROM note_provenance")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |row| row.get::<_, String>(0))
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    pub fn set_provenance_state(
        &self,
        note_id: &str,
        state: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "UPDATE note_provenance SET state = ?2, modified_at = ?3
             WHERE id = ?1",
            rusqlite::params![note_id, state, now_iso()],
        )
        .map_err(|e| format!("provenance state: {e}"))?;
        Ok(())
    }
}

impl DbTx<'_> {
    pub fn get_share(&self, source_id: &str) -> Option<LocalShare> {
        let conn = self.0;
        conn.query_row(
            "SELECT id, kind, code, expires_at, created_at
             FROM note_shares WHERE id = ?1",
            [source_id],
            |row| {
                Ok(LocalShare {
                    source_id: row.get(0)?,
                    kind: ShareKind::parse(&row.get::<_, String>(1)?)
                        .unwrap_or(ShareKind::Note),
                    code: row.get(2)?,
                    expires_at: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .ok()
    }

    // Drop my share record (after a revoke, or when the source is deleted).
    pub fn delete_share(&self, source_id: &str) -> Result<(), String> {
        self.savepoint("delete_share", |tx| {
            sync_meta::tombstone_entity(tx, "note_share", source_id)?;
            tx.execute("DELETE FROM note_shares WHERE id = ?1", [source_id])
                .map_err(|e| format!("delete share: {e}"))?;
            Ok(())
        })
    }

    pub fn delete_provenance(&self, note_id: &str) -> Result<(), String> {
        self.savepoint("delete_provenance", |tx| {
            sync_meta::tombstone_entity(tx, "note_provenance", note_id)?;
            tx.execute("DELETE FROM note_provenance WHERE id = ?1", [note_id])
                .map_err(|e| format!("delete provenance: {e}"))?;
            Ok(())
        })
    }
}

impl Database {
    pub fn get_share(&self, source_id: &str) -> Option<LocalShare> {
        DbTx(&self.conn()).get_share(source_id)
    }

    pub fn delete_share(&self, source_id: &str) -> Result<(), String> {
        DbTx(&self.conn()).delete_share(source_id)
    }

    pub fn delete_provenance(&self, note_id: &str) -> Result<(), String> {
        DbTx(&self.conn()).delete_provenance(note_id)
    }
}
