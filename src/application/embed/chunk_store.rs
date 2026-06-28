use crate::infrastructure::persistence::chunk_repo::{
    content_hash, ChunkRecord,
};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::vectordb::{Chunk, VectorStore};

use super::log;

// Chunks have no tracking triggers (they travel inside their owner's sync
// payload), so after writing fresh BLOBs the owner's sync meta must be bumped
// for the new vectors to propagate. The alive-check and the
// bump run in ONE IMMEDIATE transaction: the write lock is taken before the
// check, so a delete committing on another connection can never slip between
// them (an embed finishing after a delete would otherwise resurrect the
// tombstone on peers), and the seq allocation inside the bump stays atomic
// across connections. A successful bump pokes the sync engine: the embed
// thread finishes AFTER the save-triggered push, so without its own trigger
// the fresh vectors would sit stranded until an unrelated event (and the
// earlier chunkless push would have wiped the peer's previous vectors).
pub(crate) fn bump_owner_meta_if_alive(
    db: &Database,
    owner_id: &str,
    owner_kind: &str,
) {
    let exists_sql = match owner_kind {
        "note" => "SELECT EXISTS(SELECT 1 FROM notes WHERE id = ?1)",
        "attachment" => {
            "SELECT EXISTS(SELECT 1 FROM attachments WHERE id = ?1)"
        }
        _ => return,
    };
    let conn = db.conn();
    if let Err(e) = conn.execute_batch("BEGIN IMMEDIATE;") {
        log(&format!("chunk meta bump tx: {e}"));
        return;
    }
    let alive: bool = conn
        .query_row(exists_sql, [owner_id], |r| r.get(0))
        .unwrap_or(false);
    let res = if alive {
        crate::infrastructure::persistence::sync_meta::mark_entity_updated(
            &conn, owner_kind, owner_id,
        )
    } else {
        Ok(())
    };
    match res {
        Ok(()) => {
            if let Err(e) = conn.execute_batch("COMMIT;") {
                log(&format!("chunk meta bump commit: {e}"));
                let _ = conn.execute_batch("ROLLBACK;");
            } else if alive {
                drop(conn);
                crate::infrastructure::sync::engine::poke_after_data_change();
            }
        }
        Err(e) => {
            log(&format!("chunk meta bump: {e}"));
            let _ = conn.execute_batch("ROLLBACK;");
        }
    }
}

pub(crate) fn persist_chunk_blobs(
    owner_id: &str,
    owner_kind: &str,
    entries: &[Chunk],
) {
    let records: Vec<ChunkRecord> = entries
        .iter()
        .map(|c| ChunkRecord {
            id: c.id.clone(),
            owner_id: owner_id.to_string(),
            owner_kind: owner_kind.to_string(),
            chunk_index: c.chunk_index,
            vector: c.vector.clone(),
            content_hash: content_hash(&c.chunk_text),
            chunk_text: c.chunk_text.clone(),
            title: c.title.clone(),
            tags: c.tags.clone(),
            created_at: c.created_at.clone(),
        })
        .collect();
    match crate::infrastructure::persistence::Database::open() {
        Ok(db) => {
            if let Err(e) = db.replace_chunks(owner_id, owner_kind, &records) {
                log(&format!("chunk blob persist: {e}"));
            } else {
                bump_owner_meta_if_alive(&db, owner_id, owner_kind);
            }
        }
        Err(e) => log(&format!("chunk blob db open: {e}")),
    }
}

pub(crate) async fn purge_owner_chunks(owner_id: &str, owner_kind: &str) {
    let mut had_blobs = false;
    if let Ok(db) = crate::infrastructure::persistence::Database::open() {
        had_blobs =
            db.count_chunks_for_owner(owner_id, owner_kind).unwrap_or(0) > 0;
        let _ = db.delete_owner_chunks(owner_id, owner_kind);
        if had_blobs {
            bump_owner_meta_if_alive(&db, owner_id, owner_kind);
        }
    }
    if had_blobs {
        if let Ok(store) = VectorStore::open().await {
            match owner_kind {
                "attachment" => {
                    let _ = store.delete_attachment_chunks(owner_id).await;
                }
                _ => {
                    let _ = store.delete_note_own_chunks(owner_id).await;
                }
            }
        }
    }
}
