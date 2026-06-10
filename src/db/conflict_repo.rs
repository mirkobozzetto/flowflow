use crate::db::Database;

// One archived losing version of a sync merge (RFC 0004 T18). The snapshot
// holds the entity's fields as JSON; losing_vector_ref holds its archived
// chunk vectors (see services::sync::conflict::ArchivedChunk).
#[derive(Debug, Clone, PartialEq)]
pub struct SyncConflict {
    pub id: i64,
    pub entity_kind: String,
    pub entity_id: String,
    pub losing_vv: String,
    pub losing_snapshot_json: String,
    pub losing_vector_ref: Option<String>,
    pub created_hlc: Option<String>,
}

fn row_to_conflict(row: &rusqlite::Row) -> rusqlite::Result<SyncConflict> {
    Ok(SyncConflict {
        id: row.get("id")?,
        entity_kind: row.get("entity_kind")?,
        entity_id: row.get("entity_id")?,
        losing_vv: row.get("losing_vv")?,
        losing_snapshot_json: row.get("losing_snapshot_json")?,
        losing_vector_ref: row.get("losing_vector_ref")?,
        created_hlc: row.get("created_hlc")?,
    })
}

impl Database {
    pub fn list_unresolved_conflicts(
        &self,
    ) -> Result<Vec<SyncConflict>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, entity_kind, entity_id, losing_vv,
                        losing_snapshot_json, losing_vector_ref, created_hlc
                 FROM sync_conflicts WHERE resolved = 0
                 ORDER BY id DESC",
            )
            .map_err(|e| format!("list conflicts: {e}"))?;
        let conflicts = stmt
            .query_map([], row_to_conflict)
            .map_err(|e| format!("list conflicts: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list conflicts: {e}"))?;
        Ok(conflicts)
    }

    pub fn count_unresolved_conflicts(&self) -> Result<usize, String> {
        let n: i64 = self
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sync_conflicts WHERE resolved = 0",
                [],
                |r| r.get(0),
            )
            .map_err(|e| format!("count conflicts: {e}"))?;
        Ok(n as usize)
    }

    // Claim-style resolve: only an UNRESOLVED conflict flips, so two racing
    // resolutions (double-tap, stale list) cannot both proceed - the loser
    // gets an error instead of duplicating the restore's side effects.
    pub fn resolve_conflict(&self, conflict_id: i64) -> Result<(), String> {
        let changed = self
            .conn()
            .execute(
                "UPDATE sync_conflicts SET resolved = 1
                 WHERE id = ?1 AND resolved = 0",
                [conflict_id],
            )
            .map_err(|e| format!("resolve conflict: {e}"))?;
        if changed == 0 {
            return Err(format!(
                "conflict {conflict_id} not found or already resolved"
            ));
        }
        Ok(())
    }

    // Roll a claim back when the work that followed it failed, so the
    // conflict stays visible instead of vanishing with nothing restored.
    pub fn unresolve_conflict(&self, conflict_id: i64) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE sync_conflicts SET resolved = 0 WHERE id = ?1",
                [conflict_id],
            )
            .map(|_| ())
            .map_err(|e| format!("unresolve conflict: {e}"))
    }
}
