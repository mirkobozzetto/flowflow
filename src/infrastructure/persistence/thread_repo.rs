use crate::domain::{NewThread, Note, Thread, UpdateThread};
use crate::infrastructure::persistence::note_repo::row_to_note;
use crate::infrastructure::persistence::{now_iso, sync_meta, Database};
use uuid::Uuid;

fn row_to_thread(row: &rusqlite::Row) -> rusqlite::Result<Thread> {
    Ok(Thread {
        id: row.get("id")?,
        title: row.get("title")?,
        folder_id: row.get("folder_id")?,
        created_at: row.get("created_at")?,
        modified_at: row.get("modified_at")?,
    })
}

impl Database {
    pub fn create_thread(&self, thread: &NewThread) -> Result<Thread, String> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        self.conn()
            .execute(
                "INSERT INTO threads (id, title, folder_id, created_at, modified_at)
                 VALUES (?1,?2,?3,?4,?5)",
                rusqlite::params![id, thread.title, thread.folder_id, now, now],
            )
            .map_err(|e| format!("Insert thread: {e}"))?;
        self.get_thread(&id)?
            .ok_or_else(|| "Thread not found after insert".into())
    }

    pub fn get_thread(&self, id: &str) -> Result<Option<Thread>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM threads WHERE id = ?1")
            .map_err(|e| format!("Prepare: {e}"))?;
        let mut rows = stmt
            .query_map([id], row_to_thread)
            .map_err(|e| format!("Query: {e}"))?;
        match rows.next() {
            Some(Ok(t)) => Ok(Some(t)),
            Some(Err(e)) => Err(format!("Row: {e}")),
            None => Ok(None),
        }
    }

    pub fn list_threads(&self) -> Result<Vec<Thread>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM threads ORDER BY modified_at DESC")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], row_to_thread)
            .map_err(|e| format!("Query: {e}"))?;
        let mut threads = Vec::new();
        for row in rows {
            threads.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(threads)
    }

    // A thread is only a thread with >= 2 members; the feed shows those as
    // thread cards, singletons render as their flat note.
    pub fn list_feed_threads(&self) -> Result<Vec<Thread>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT t.* FROM threads t
                 WHERE (SELECT COUNT(*) FROM notes n WHERE n.thread_id = t.id) >= 2
                 ORDER BY t.modified_at DESC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], row_to_thread)
            .map_err(|e| format!("Query: {e}"))?;
        let mut threads = Vec::new();
        for row in rows {
            threads.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(threads)
    }

    // Same >= 2 global rule, scoped to a folder: a thread surfaces in a folder
    // when at least one of its members lives there.
    pub fn list_feed_threads_in_folder(
        &self,
        folder_id: &str,
    ) -> Result<Vec<Thread>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT t.* FROM threads t
                 WHERE (SELECT COUNT(*) FROM notes n WHERE n.thread_id = t.id) >= 2
                   AND EXISTS (
                       SELECT 1 FROM notes n
                       JOIN notes_folders nf ON nf.note_id = n.id
                       WHERE n.thread_id = t.id AND nf.folder_id = ?1
                   )
                 ORDER BY t.modified_at DESC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([folder_id], row_to_thread)
            .map_err(|e| format!("Query: {e}"))?;
        let mut threads = Vec::new();
        for row in rows {
            threads.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(threads)
    }

    // A thread with fewer than 2 members is not a thread: delete it, returning
    // any lone member to a flat note. Explicit, local-only cleanup: never run it
    // automatically during the sync-convergence window. Membership is split
    // across two synced rows (the thread row + each note's thread_id), so a
    // still-converging thread can look like a singleton locally while the peer
    // holds it whole; collapsing then deletes a live thread and spawns spurious
    // conflicts (RFC 0007). Singletons are already hidden by list_root_notes* /
    // list_feed_threads filtering, so no auto-collapse is needed for display.
    pub fn collapse_singleton_threads(&self) -> Result<(), String> {
        let ids: Vec<String> = {
            let conn = self.conn();
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM threads t
                     WHERE (SELECT COUNT(*) FROM notes n WHERE n.thread_id = t.id) < 2",
                )
                .map_err(|e| format!("Prepare: {e}"))?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| format!("Query: {e}"))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        for id in ids {
            self.delete_thread(&id)?;
        }
        Ok(())
    }

    pub fn list_thread_notes(
        &self,
        thread_id: &str,
    ) -> Result<Vec<Note>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM notes WHERE thread_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([thread_id], row_to_note)
            .map_err(|e| format!("Query: {e}"))?;
        let mut notes = Vec::new();
        for row in rows {
            notes.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(notes)
    }

    pub fn update_thread(
        &self,
        id: &str,
        update: &UpdateThread,
    ) -> Result<(), String> {
        let conn = self.conn();
        let now = now_iso();
        if let Some(ref title) = update.title {
            conn.execute(
                "UPDATE threads SET title = ?1, modified_at = ?2 WHERE id = ?3",
                rusqlite::params![title, now, id],
            )
            .map_err(|e| format!("Update thread title: {e}"))?;
        }
        if let Some(ref folder_id) = update.folder_id {
            conn.execute(
                "UPDATE threads SET folder_id = ?1, modified_at = ?2 WHERE id = ?3",
                rusqlite::params![folder_id, now, id],
            )
            .map_err(|e| format!("Update thread folder: {e}"))?;
        }
        Ok(())
    }

    pub fn touch_thread(&self, id: &str) -> Result<(), String> {
        let now = now_iso();
        self.conn()
            .execute(
                "UPDATE threads SET modified_at = ?1 WHERE id = ?2",
                rusqlite::params![now, id],
            )
            .map_err(|e| format!("Touch thread: {e}"))?;
        Ok(())
    }

    pub fn add_note_to_thread(
        &self,
        note_id: &str,
        thread_id: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        let now = now_iso();
        conn.execute(
            "UPDATE notes SET thread_id = ?1, modified_at = ?2 WHERE id = ?3",
            rusqlite::params![thread_id, now, note_id],
        )
        .map_err(|e| format!("Add note to thread: {e}"))?;
        conn.execute(
            "UPDATE threads SET modified_at = ?1 WHERE id = ?2",
            rusqlite::params![now, thread_id],
        )
        .map_err(|e| format!("Touch thread on add: {e}"))?;
        Ok(())
    }

    pub fn remove_note_from_thread(&self, note_id: &str) -> Result<(), String> {
        let now = now_iso();
        self.conn()
            .execute(
                "UPDATE notes SET thread_id = NULL, modified_at = ?1 WHERE id = ?2",
                rusqlite::params![now, note_id],
            )
            .map_err(|e| format!("Remove note from thread: {e}"))?;
        Ok(())
    }

    pub fn delete_thread(&self, id: &str) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Delete thread tx: {e}"))?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM threads WHERE id = ?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Delete thread check: {e}"))?;
        if !exists {
            return Ok(());
        }
        let now = now_iso();
        tx.execute(
            "UPDATE notes SET thread_id = NULL, modified_at = ?1 WHERE thread_id = ?2",
            rusqlite::params![now, id],
        )
        .map_err(|e| format!("Orphan thread members: {e}"))?;
        tx.execute(
            "DELETE FROM settings WHERE key LIKE 'chat_scope:%' AND value = ?1",
            [format!("thread:{id}")],
        )
        .map_err(|e| format!("Clear thread scope rows: {e}"))?;
        sync_meta::tombstone_entity(&tx, "thread", id)?;
        tx.execute("DELETE FROM threads WHERE id = ?1", [id])
            .map_err(|e| format!("Delete thread: {e}"))?;
        tx.commit()
            .map_err(|e| format!("Delete thread commit: {e}"))?;
        Ok(())
    }
}

impl crate::infrastructure::persistence::DbTx<'_> {
    /// Mirror a remote thread under its own id: the space plane and the local
    /// store share thread ids, so a pull never needs a mapping table.
    pub fn upsert_thread_with_id(
        &self,
        id: &str,
        title: &str,
        folder_id: Option<&str>,
    ) -> Result<(), String> {
        let now = now_iso();
        self.0
            .execute(
                "INSERT INTO threads (id, title, folder_id, created_at, modified_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   folder_id = excluded.folder_id,
                   modified_at = excluded.modified_at",
                rusqlite::params![id, title, folder_id, now],
            )
            .map_err(|e| format!("Upsert thread: {e}"))?;
        Ok(())
    }

    pub fn thread_exists(&self, id: &str) -> Result<bool, String> {
        self.0
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM threads WHERE id = ?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Thread exists: {e}"))
    }

    /// Set or clear a note's thread without touching its content.
    pub fn set_note_thread(
        &self,
        note_id: &str,
        thread_id: Option<&str>,
    ) -> Result<(), String> {
        self.0
            .execute(
                "UPDATE notes SET thread_id = ?1 WHERE id = ?2",
                rusqlite::params![thread_id, note_id],
            )
            .map_err(|e| format!("Set note thread: {e}"))?;
        if let Some(thread_id) = thread_id {
            self.0
                .execute(
                    "UPDATE threads SET modified_at = ?1 WHERE id = ?2",
                    rusqlite::params![now_iso(), thread_id],
                )
                .map_err(|e| format!("Touch thread: {e}"))?;
        }
        Ok(())
    }

    /// Drop a thread the space tombstoned. Members are detached, never deleted.
    pub fn delete_thread(&self, id: &str) -> Result<(), String> {
        let now = now_iso();
        self.0
            .execute(
                "UPDATE notes SET thread_id = NULL, modified_at = ?1 WHERE thread_id = ?2",
                rusqlite::params![now, id],
            )
            .map_err(|e| format!("Orphan thread members: {e}"))?;
        self.0
            .execute(
                "DELETE FROM settings WHERE key LIKE 'chat_scope:%' AND value = ?1",
                [format!("thread:{id}")],
            )
            .map_err(|e| format!("Clear thread scope rows: {e}"))?;
        sync_meta::tombstone_entity(self.0, "thread", id)?;
        self.0
            .execute("DELETE FROM threads WHERE id = ?1", [id])
            .map_err(|e| format!("Delete thread: {e}"))?;
        Ok(())
    }
}
