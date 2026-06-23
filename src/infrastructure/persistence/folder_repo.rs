use crate::domain::{Folder, NewFolder, UpdateFolder};
use crate::infrastructure::persistence::{now_iso, sync_meta, Database};
use uuid::Uuid;

fn row_to_folder(row: &rusqlite::Row) -> rusqlite::Result<Folder> {
    Ok(Folder {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        parent_id: row.get("parent_id")?,
        created_at: row.get("created_at")?,
        modified_at: row.get("modified_at")?,
    })
}

impl Database {
    pub fn create_folder(&self, folder: &NewFolder) -> Result<Folder, String> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        self.conn()
            .execute(
                "INSERT INTO folders (id, name, description, parent_id, created_at, modified_at)
                 VALUES (?1,?2,?3,?4,?5,?6)",
                rusqlite::params![id, folder.name, folder.description, folder.parent_id, now, now],
            )
            .map_err(|e| format!("Insert folder: {e}"))?;
        self.get_folder(&id)?
            .ok_or_else(|| "Folder not found after insert".into())
    }

    pub fn get_folder(&self, id: &str) -> Result<Option<Folder>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM folders WHERE id = ?1")
            .map_err(|e| format!("Prepare: {e}"))?;
        let mut rows = stmt
            .query_map([id], row_to_folder)
            .map_err(|e| format!("Query: {e}"))?;
        match rows.next() {
            Some(Ok(f)) => Ok(Some(f)),
            Some(Err(e)) => Err(format!("Row: {e}")),
            None => Ok(None),
        }
    }

    pub fn list_all_folders(&self) -> Result<Vec<Folder>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM folders ORDER BY name ASC")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], row_to_folder)
            .map_err(|e| format!("Query: {e}"))?;
        let mut folders = Vec::new();
        for row in rows {
            folders.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(folders)
    }

    pub fn list_root_folders(&self) -> Result<Vec<Folder>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT * FROM folders WHERE parent_id IS NULL ORDER BY name ASC")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], row_to_folder)
            .map_err(|e| format!("Query: {e}"))?;
        let mut folders = Vec::new();
        for row in rows {
            folders.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(folders)
    }

    pub fn list_subfolders(
        &self,
        parent_id: &str,
    ) -> Result<Vec<Folder>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM folders WHERE parent_id = ?1 ORDER BY name ASC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([parent_id], row_to_folder)
            .map_err(|e| format!("Query: {e}"))?;
        let mut folders = Vec::new();
        for row in rows {
            folders.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(folders)
    }

    pub fn update_folder(
        &self,
        id: &str,
        update: &UpdateFolder,
    ) -> Result<(), String> {
        let conn = self.conn();
        let now = now_iso();
        if let Some(ref name) = update.name {
            conn.execute(
                "UPDATE folders SET name = ?1, modified_at = ?2 WHERE id = ?3",
                rusqlite::params![name, now, id],
            )
            .map_err(|e| format!("Update name: {e}"))?;
        }
        if let Some(ref desc) = update.description {
            conn.execute(
                "UPDATE folders SET description = ?1, modified_at = ?2 WHERE id = ?3",
                rusqlite::params![desc, now, id],
            )
            .map_err(|e| format!("Update desc: {e}"))?;
        }
        if let Some(ref parent) = update.parent_id {
            conn.execute(
                "UPDATE folders SET parent_id = ?1, modified_at = ?2 WHERE id = ?3",
                rusqlite::params![parent, now, id],
            )
            .map_err(|e| format!("Update parent: {e}"))?;
        }
        Ok(())
    }

    pub fn delete_folder(&self, id: &str) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Delete folder tx: {e}"))?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE id = ?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Delete folder check: {e}"))?;
        if !exists {
            return Ok(());
        }
        // notes_folders links are CASCADE-deleted: tombstone them before the row.
        for link in sync_meta::collect_links(
            &tx,
            "SELECT folder_id, note_id FROM notes_folders WHERE folder_id = ?1",
            id,
        ) {
            sync_meta::tombstone_entity(&tx, "notes_folders", &link)?;
        }
        sync_meta::tombstone_entity(&tx, "folder", id)?;
        // Subfolders are reparented to NULL by ON DELETE SET NULL; with
        // recursive_triggers=ON that fires the folders AFTER UPDATE trigger, which
        // records the reparent automatically (no manual mark; covered by a test).
        tx.execute("DELETE FROM folders WHERE id = ?1", [id])
            .map_err(|e| format!("Delete folder: {e}"))?;
        tx.commit()
            .map_err(|e| format!("Delete folder commit: {e}"))?;
        Ok(())
    }

    pub fn add_note_to_folder(
        &self,
        note_id: &str,
        folder_id: &str,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT OR IGNORE INTO notes_folders (note_id, folder_id) VALUES (?1, ?2)",
                rusqlite::params![note_id, folder_id],
            )
            .map_err(|e| format!("Link note-folder: {e}"))?;
        Ok(())
    }

    pub fn remove_note_from_folder(
        &self,
        note_id: &str,
        folder_id: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Unlink tx: {e}"))?;
        let n = tx
            .execute(
                "DELETE FROM notes_folders WHERE note_id = ?1 AND folder_id = ?2",
                rusqlite::params![note_id, folder_id],
            )
            .map_err(|e| format!("Unlink note-folder: {e}"))?;
        if n > 0 {
            sync_meta::tombstone_entity(
                &tx,
                "notes_folders",
                &format!("{folder_id}:{note_id}"),
            )?;
        }
        tx.commit().map_err(|e| format!("Unlink commit: {e}"))?;
        Ok(())
    }

    pub fn folders_for_note(
        &self,
        note_id: &str,
    ) -> Result<Vec<Folder>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT f.* FROM folders f
                 JOIN notes_folders nf ON nf.folder_id = f.id
                 WHERE nf.note_id = ?1
                 ORDER BY f.name ASC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([note_id], row_to_folder)
            .map_err(|e| format!("Query: {e}"))?;
        let mut folders = Vec::new();
        for row in rows {
            folders.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(folders)
    }
}
