// Local persistence of collaborative spaces (proposal 0002 T09): the joined
// spaces with their pull cursor, and the remote_id -> local id lookups the
// delta applier needs.
//
// Space folders and notes are ORDINARY rows in `folders` and `notes`, tagged
// with `space_id` + `remote_id`. That is what makes a received note searchable
// and usable in chat with no extra code: it goes through the same repos, the
// same embed pipeline, the same chat surface as anything the user wrote.

use crate::domain::space::{publish_backoff, Space};
use crate::infrastructure::persistence::{iso_at, now_iso, Database};
use rusqlite::{Connection, OptionalExtension};

/// The connection inside a unit of work. Repo methods that a transaction may
/// need live here; the matching `Database` methods delegate to them, so one
/// SQL body serves both the one-off call and the page-wide transaction.
pub struct DbTx<'a>(pub(crate) &'a Connection);

/// A note staged for publication to its space, with its retry state.
#[derive(Debug, Clone, PartialEq)]
pub struct PublishPending {
    pub note_id: String,
    pub space_id: String,
    pub attempts: i64,
    pub next_try_at: String,
    pub last_error: Option<String>,
}

impl DbTx<'_> {
    /// A nested-safe unit of work. SAVEPOINT / RELEASE is valid inside and
    /// outside an outer transaction, where a BEGIN would fail and a dropped
    /// rusqlite `Transaction` would roll the whole outer page back.
    pub(crate) fn savepoint<T>(
        &self,
        name: &str,
        f: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        self.0
            .execute_batch(&format!("SAVEPOINT {name}"))
            .map_err(|e| format!("{name} savepoint: {e}"))?;
        match f(self.0) {
            Ok(v) => {
                self.0
                    .execute_batch(&format!("RELEASE {name}"))
                    .map_err(|e| format!("{name} release: {e}"))?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.0.execute_batch(&format!(
                    "ROLLBACK TO {name}; RELEASE {name}"
                ));
                Err(e)
            }
        }
    }
}

impl Database {
    /// Hold the connection for a run of repo calls. No transaction of its own:
    /// each call still commits on its own, this only spares the re-locking.
    pub fn with_tx<T>(&self, f: impl FnOnce(&DbTx) -> T) -> T {
        let conn = self.conn();
        f(&DbTx(&conn))
    }

    /// One page of delta = one transaction that also carries the cursor.
    /// Any error rolls the page back with the cursor untouched, so the server
    /// replays it: rows never half-land and never get skipped. The connection
    /// mutex is held for the whole page, so `f` must only use the `DbTx` it is
    /// handed, never `Database` (that would deadlock).
    pub fn apply_space_page<T>(
        &self,
        space_id: &str,
        next_seq: i64,
        f: impl FnOnce(&DbTx) -> Result<T, String>,
    ) -> Result<T, String> {
        let conn = self.conn();
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("Space page begin: {e}"))?;
        let tx = DbTx(&conn);
        let res = f(&tx).and_then(|t| {
            tx.set_space_cursor(space_id, next_seq)?;
            Ok(t)
        });
        let res = res.and_then(|t| {
            conn.execute_batch("COMMIT")
                .map_err(|e| format!("Space page commit: {e}"))?;
            Ok(t)
        });
        if res.is_err() {
            let _ = conn.execute_batch("ROLLBACK");
        }
        res
    }
}

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

    /// Forget a space locally. The content rows are NOT touched here: leaving
    /// with "keep my notes" detaches them first, and leaving without it deletes
    /// them first. Both are use-case decisions, not a repo's.
    pub fn delete_space(&self, id: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM spaces WHERE id = ?1", [id])
            .map_err(|e| format!("Delete space: {e}"))?;
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

impl DbTx<'_> {
    pub fn local_folder_for_remote(
        &self,
        space_id: &str,
        remote_id: &str,
    ) -> Option<String> {
        self.0
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
        self.0
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
        self.0
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
        self.0
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

    /// Advance the cursor after a delta has been fully applied. Never call it
    /// before: a cursor moved past rows that failed to apply loses them for
    /// good, the server has no way to replay what the client claims to have.
    pub fn set_space_cursor(
        &self,
        id: &str,
        cursor: i64,
    ) -> Result<(), String> {
        self.0
            .execute(
                "UPDATE spaces SET cursor = ?1, last_pull_at = ?2
                 WHERE id = ?3",
                rusqlite::params![cursor, now_iso(), id],
            )
            .map_err(|e| format!("Set space cursor: {e}"))?;
        Ok(())
    }

    /// Cut a note loose from its space: it becomes an ordinary local note,
    /// waiting for no pull. This is what "keep my notes" does on the way out.
    pub fn detach_note_from_space(&self, local_id: &str) -> Result<(), String> {
        self.0
            .execute(
                "UPDATE notes SET space_id = NULL, remote_id = NULL,
                     modified_at = ?1
                 WHERE id = ?2",
                rusqlite::params![now_iso(), local_id],
            )
            .map_err(|e| format!("Detach note: {e}"))?;
        Ok(())
    }
}

impl Database {
    pub fn local_folder_for_remote(
        &self,
        space_id: &str,
        remote_id: &str,
    ) -> Option<String> {
        DbTx(&self.conn()).local_folder_for_remote(space_id, remote_id)
    }

    pub fn local_note_for_remote(
        &self,
        space_id: &str,
        remote_id: &str,
    ) -> Option<String> {
        DbTx(&self.conn()).local_note_for_remote(space_id, remote_id)
    }

    pub fn mark_folder_in_space(
        &self,
        local_id: &str,
        space_id: &str,
        remote_id: &str,
        mode: &str,
    ) -> Result<(), String> {
        DbTx(&self.conn())
            .mark_folder_in_space(local_id, space_id, remote_id, mode)
    }

    pub fn mark_note_in_space(
        &self,
        local_id: &str,
        space_id: &str,
        remote_id: &str,
        author_ref: Option<&str>,
    ) -> Result<(), String> {
        DbTx(&self.conn())
            .mark_note_in_space(local_id, space_id, remote_id, author_ref)
    }

    pub fn set_space_cursor(
        &self,
        id: &str,
        cursor: i64,
    ) -> Result<(), String> {
        DbTx(&self.conn()).set_space_cursor(id, cursor)
    }

    pub fn detach_note_from_space(&self, local_id: &str) -> Result<(), String> {
        DbTx(&self.conn()).detach_note_from_space(local_id)
    }
}

// ---- bounded republication of notes saved into a space (space_publish_pending) ----

impl DbTx<'_> {
    /// Bind a local note to the remote id it will be published under AND queue
    /// it, in one unit: a note either carries its id and waits for the push, or
    /// neither. The row is kept if it already exists.
    pub fn stage_note_publish(
        &self,
        local_id: &str,
        space_id: &str,
        remote_id: &str,
        author_ref: Option<&str>,
    ) -> Result<(), String> {
        self.savepoint("stage_publish", |tx| {
            self.mark_note_in_space(local_id, space_id, remote_id, author_ref)?;
            tx.execute(
                "INSERT OR IGNORE INTO space_publish_pending
                     (note_id, space_id, next_try_at)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![local_id, space_id, now_iso()],
            )
            .map_err(|e| format!("Stage publish: {e}"))?;
            Ok(())
        })
    }

    pub fn clear_note_publish(&self, note_id: &str) -> Result<(), String> {
        self.0
            .execute(
                "DELETE FROM space_publish_pending WHERE note_id = ?1",
                [note_id],
            )
            .map_err(|e| format!("Clear publish: {e}"))?;
        Ok(())
    }

    /// A transient failure: count it and push the next try out. A note that
    /// was never staged is left alone.
    pub fn defer_note_publish(
        &self,
        note_id: &str,
        error: &str,
    ) -> Result<(), String> {
        let attempts: Option<i64> = self
            .0
            .query_row(
                "SELECT attempts FROM space_publish_pending WHERE note_id = ?1",
                [note_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| format!("Publish attempts: {e}"))?;
        let Some(attempts) = attempts else {
            return Ok(());
        };
        let attempts = attempts + 1;
        let next = iso_at(chrono::Utc::now() + publish_backoff(attempts));
        self.0
            .execute(
                "UPDATE space_publish_pending
                 SET attempts = ?1, next_try_at = ?2, last_error = ?3
                 WHERE note_id = ?4",
                rusqlite::params![attempts, next, error, note_id],
            )
            .map_err(|e| format!("Defer publish: {e}"))?;
        Ok(())
    }

    /// Notes of the space whose retry time has come, oldest first.
    pub fn due_note_publishes(
        &self,
        space_id: &str,
        now: &str,
        limit: usize,
    ) -> Result<Vec<String>, String> {
        let mut stmt = self
            .0
            .prepare(
                "SELECT note_id FROM space_publish_pending
                 WHERE space_id = ?1 AND next_try_at <= ?2
                 ORDER BY next_try_at ASC LIMIT ?3",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![space_id, now, limit as i64], |r| {
                r.get(0)
            })
            .map_err(|e| format!("Query: {e}"))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(out)
    }

    pub fn note_publish_state(&self, note_id: &str) -> Option<PublishPending> {
        self.0
            .query_row(
                "SELECT note_id, space_id, attempts, next_try_at, last_error
                 FROM space_publish_pending WHERE note_id = ?1",
                [note_id],
                |r| {
                    Ok(PublishPending {
                        note_id: r.get(0)?,
                        space_id: r.get(1)?,
                        attempts: r.get(2)?,
                        next_try_at: r.get(3)?,
                        last_error: r.get(4)?,
                    })
                },
            )
            .ok()
    }
}

impl Database {
    pub fn stage_note_publish(
        &self,
        local_id: &str,
        space_id: &str,
        remote_id: &str,
        author_ref: Option<&str>,
    ) -> Result<(), String> {
        DbTx(&self.conn())
            .stage_note_publish(local_id, space_id, remote_id, author_ref)
    }

    pub fn clear_note_publish(&self, note_id: &str) -> Result<(), String> {
        DbTx(&self.conn()).clear_note_publish(note_id)
    }

    pub fn defer_note_publish(
        &self,
        note_id: &str,
        error: &str,
    ) -> Result<(), String> {
        DbTx(&self.conn()).defer_note_publish(note_id, error)
    }

    pub fn due_note_publishes(
        &self,
        space_id: &str,
        now: &str,
        limit: usize,
    ) -> Result<Vec<String>, String> {
        DbTx(&self.conn()).due_note_publishes(space_id, now, limit)
    }

    pub fn note_publish_state(&self, note_id: &str) -> Option<PublishPending> {
        DbTx(&self.conn()).note_publish_state(note_id)
    }
}
