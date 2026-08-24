// Local persistence of collaborative spaces (proposal 0002 T09): the joined
// spaces with their pull cursor, and the remote_id -> local id lookups the
// delta applier needs.
//
// Space folders and notes are ORDINARY rows in `folders` and `notes`, tagged
// with `space_id` + `remote_id`. That is what makes a received note searchable
// and usable in chat with no extra code: it goes through the same repos, the
// same embed pipeline, the same chat surface as anything the user wrote.

use crate::domain::space::Space;
use crate::infrastructure::persistence::{now_iso, Database};

fn row_to_space(row: &rusqlite::Row) -> rusqlite::Result<Space> {
    Ok(Space {
        id: row.get("id")?,
        name: row.get("name")?,
        is_owner: row.get::<_, i64>("is_owner")? != 0,
        joined_at: row.get("joined_at")?,
        cursor: row.get("cursor")?,
        last_pull_at: row.get("last_pull_at")?,
    })
}

impl Database {
    /// Record a joined space. Re-joining after a revocation keeps the row but
    /// resets the cursor to 0: the server replays everything from scratch,
    /// which is exactly what a returning member needs.
    pub fn upsert_space(
        &self,
        id: &str,
        name: &str,
        is_owner: bool,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT INTO spaces (id, name, is_owner, joined_at, cursor)
                 VALUES (?1, ?2, ?3, ?4, 0)
                 ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     is_owner = MAX(spaces.is_owner, excluded.is_owner)",
                rusqlite::params![id, name, is_owner as i64, now_iso()],
            )
            .map_err(|e| format!("Upsert space: {e}"))?;
        Ok(())
    }

    pub fn get_space(&self, id: &str) -> Result<Option<Space>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM spaces WHERE id = ?1")
            .map_err(|e| format!("Prepare: {e}"))?;
        let mut rows = stmt
            .query_map([id], row_to_space)
            .map_err(|e| format!("Query: {e}"))?;
        match rows.next() {
            Some(Ok(s)) => Ok(Some(s)),
            Some(Err(e)) => Err(format!("Row: {e}")),
            None => Ok(None),
        }
    }

    pub fn list_spaces(&self) -> Result<Vec<Space>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM spaces ORDER BY name ASC")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], row_to_space)
            .map_err(|e| format!("Query: {e}"))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(out)
    }

    /// Advance the cursor after a delta has been fully applied. Never call it
    /// before: a cursor moved past rows that failed to apply loses them for
    /// good, the server has no way to replay what the client claims to have.
    pub fn set_space_cursor(
        &self,
        id: &str,
        cursor: i64,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE spaces SET cursor = ?1, last_pull_at = ?2
                 WHERE id = ?3",
                rusqlite::params![cursor, now_iso(), id],
            )
            .map_err(|e| format!("Set space cursor: {e}"))?;
        Ok(())
    }

    /// Forget a space locally. The content rows are NOT touched here: leaving
    /// with "keep my notes" detaches them first, and leaving without it deletes
    /// them first. Both are use-case decisions, not a repo's.
    pub fn delete_space(&self, id: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM spaces WHERE id = ?1", [id])
            .map_err(|e| format!("Delete space: {e}"))?;
        Ok(())
    }

    pub fn local_folder_for_remote(
        &self,
        space_id: &str,
        remote_id: &str,
    ) -> Option<String> {
        self.conn()
            .query_row(
                "SELECT id FROM folders WHERE space_id = ?1 AND remote_id = ?2",
                rusqlite::params![space_id, remote_id],
                |r| r.get(0),
            )
            .ok()
    }

    pub fn local_note_for_remote(
        &self,
        space_id: &str,
        remote_id: &str,
    ) -> Option<String> {
        self.conn()
            .query_row(
                "SELECT id FROM notes WHERE space_id = ?1 AND remote_id = ?2",
                rusqlite::params![space_id, remote_id],
                |r| r.get(0),
            )
            .ok()
    }

    /// Tag a freshly created local folder as this space's mirror of a remote
    /// one. Kept apart from `create_folder` so the ordinary folder path stays
    /// untouched by the space plane.
    pub fn mark_folder_in_space(
        &self,
        local_id: &str,
        space_id: &str,
        remote_id: &str,
        mode: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE folders SET space_id = ?1, remote_id = ?2, mode = ?3,
                     modified_at = ?4
                 WHERE id = ?5",
                rusqlite::params![
                    space_id,
                    remote_id,
                    mode,
                    now_iso(),
                    local_id
                ],
            )
            .map_err(|e| format!("Mark folder in space: {e}"))?;
        Ok(())
    }

    pub fn mark_note_in_space(
        &self,
        local_id: &str,
        space_id: &str,
        remote_id: &str,
        author_ref: Option<&str>,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE notes SET space_id = ?1, remote_id = ?2,
                     author_ref = ?3, modified_at = ?4
                 WHERE id = ?5",
                rusqlite::params![
                    space_id,
                    remote_id,
                    author_ref,
                    now_iso(),
                    local_id
                ],
            )
            .map_err(|e| format!("Mark note in space: {e}"))?;
        Ok(())
    }

    /// Cut a note loose from its space: it becomes an ordinary local note,
    /// waiting for no pull. This is what "keep my notes" does on the way out.
    pub fn detach_note_from_space(&self, local_id: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE notes SET space_id = NULL, remote_id = NULL,
                     modified_at = ?1
                 WHERE id = ?2",
                rusqlite::params![now_iso(), local_id],
            )
            .map_err(|e| format!("Detach note: {e}"))?;
        Ok(())
    }

    pub fn detach_folder_from_space(
        &self,
        local_id: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE folders SET space_id = NULL, remote_id = NULL,
                     mode = NULL, modified_at = ?1
                 WHERE id = ?2",
                rusqlite::params![now_iso(), local_id],
            )
            .map_err(|e| format!("Detach folder: {e}"))?;
        Ok(())
    }

    /// The space's folder tree as the domain sees it: declared mode + parent,
    /// keyed by REMOTE id, because that is the id the server reasons about.
    pub fn space_folder_tree(
        &self,
        space_id: &str,
    ) -> Result<Vec<crate::domain::space::SpaceFolder>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT f.remote_id AS id, p.remote_id AS parent_id,
                        COALESCE(f.mode, 'read') AS mode
                 FROM folders f
                 LEFT JOIN folders p ON p.id = f.parent_id
                 WHERE f.space_id = ?1 AND f.remote_id IS NOT NULL",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([space_id], |r| {
                Ok(crate::domain::space::SpaceFolder {
                    id: r.get("id")?,
                    parent_id: r.get("parent_id")?,
                    mode: r.get("mode")?,
                })
            })
            .map_err(|e| format!("Query: {e}"))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(out)
    }

    /// Local ids of every note this space holds, authored by anyone.
    pub fn space_note_ids(
        &self,
        space_id: &str,
    ) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id FROM notes WHERE space_id = ?1")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([space_id], |r| r.get(0))
            .map_err(|e| format!("Query: {e}"))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(out)
    }

    pub fn space_folder_ids(
        &self,
        space_id: &str,
    ) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id FROM folders WHERE space_id = ?1")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([space_id], |r| r.get(0))
            .map_err(|e| format!("Query: {e}"))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(out)
    }
}
