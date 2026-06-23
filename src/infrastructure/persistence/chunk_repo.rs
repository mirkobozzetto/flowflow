use crate::infrastructure::persistence::Database;

pub struct ChunkRecord {
    pub id: String,
    pub owner_id: String,
    pub owner_kind: String,
    pub chunk_index: i32,
    pub vector: Vec<f32>,
    pub content_hash: String,
    pub chunk_text: String,
    pub title: String,
    pub tags: String,
    pub created_at: String,
}

pub(crate) fn content_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub fn vector_to_blob(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for f in v {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

pub fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

pub(crate) fn delete_chunks_for_owner(
    conn: &rusqlite::Connection,
    owner_id: &str,
    owner_kind: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM chunks WHERE owner_id = ?1 AND owner_kind = ?2",
        rusqlite::params![owner_id, owner_kind],
    )
}

impl Database {
    pub fn delete_owner_chunks(
        &self,
        owner_id: &str,
        owner_kind: &str,
    ) -> Result<(), String> {
        delete_chunks_for_owner(&self.conn(), owner_id, owner_kind)
            .map(|_| ())
            .map_err(|e| format!("Delete owner chunks: {e}"))
    }

    pub fn replace_chunks(
        &self,
        owner_id: &str,
        owner_kind: &str,
        chunks: &[ChunkRecord],
    ) -> Result<(), String> {
        let conn = self.conn();
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Chunks tx: {e}"))?;
        tx.execute(
            "DELETE FROM chunks WHERE owner_id = ?1 AND owner_kind = ?2",
            rusqlite::params![owner_id, owner_kind],
        )
        .map_err(|e| format!("Delete chunks: {e}"))?;
        for c in chunks {
            let blob = vector_to_blob(&c.vector);
            tx.execute(
                "INSERT OR REPLACE INTO chunks
                 (id, owner_id, owner_kind, chunk_index, dim, vector,
                  content_hash, chunk_text, title, tags, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                rusqlite::params![
                    c.id,
                    c.owner_id,
                    c.owner_kind,
                    c.chunk_index,
                    c.vector.len() as i64,
                    blob,
                    c.content_hash,
                    c.chunk_text,
                    c.title,
                    c.tags,
                    c.created_at,
                ],
            )
            .map_err(|e| format!("Insert chunk: {e}"))?;
        }
        tx.commit().map_err(|e| format!("Chunks commit: {e}"))?;
        Ok(())
    }

    pub fn count_chunks_for_owner(
        &self,
        owner_id: &str,
        owner_kind: &str,
    ) -> Result<usize, String> {
        let conn = self.conn();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chunks
                 WHERE owner_id = ?1 AND owner_kind = ?2",
                rusqlite::params![owner_id, owner_kind],
                |r| r.get(0),
            )
            .map_err(|e| format!("Count chunks: {e}"))?;
        Ok(n as usize)
    }

    pub fn all_chunk_ids(&self) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id FROM chunks")
            .map_err(|e| format!("Prepare all ids: {e}"))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| format!("Query all ids: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("Id row: {e}"))?);
        }
        Ok(out)
    }

    pub fn distinct_chunk_owners(
        &self,
    ) -> Result<Vec<(String, String)>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT DISTINCT owner_id, owner_kind FROM chunks")
            .map_err(|e| format!("Prepare owners: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Query owners: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("Owner row: {e}"))?);
        }
        Ok(out)
    }

    pub fn chunks_for_owner(
        &self,
        owner_id: &str,
        owner_kind: &str,
    ) -> Result<Vec<ChunkRecord>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, owner_id, owner_kind, chunk_index, vector,
                        content_hash, chunk_text, title, tags, created_at
                 FROM chunks
                 WHERE owner_id = ?1 AND owner_kind = ?2
                 ORDER BY chunk_index ASC",
            )
            .map_err(|e| format!("Prepare chunks: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![owner_id, owner_kind], |row| {
                let blob: Vec<u8> = row.get(4)?;
                Ok(ChunkRecord {
                    id: row.get(0)?,
                    owner_id: row.get(1)?,
                    owner_kind: row.get(2)?,
                    chunk_index: row.get(3)?,
                    vector: blob_to_vector(&blob),
                    content_hash: row.get(5)?,
                    chunk_text: row.get(6)?,
                    title: row.get(7)?,
                    tags: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| format!("Query chunks: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("Chunk row: {e}"))?);
        }
        Ok(out)
    }
}
