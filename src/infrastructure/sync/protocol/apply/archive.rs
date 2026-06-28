use super::super::collect::load_chunks;
use super::super::wire::ChunkPayload;
use crate::infrastructure::sync::conflict::ArchivedChunk;
use crate::infrastructure::sync::SyncError;
use rusqlite::Connection;

// The losing version's chunks, ready for the conflict archive. From the
// pushed payload when the REMOTE version loses; read back from the local
// chunks table (inside the apply tx, before the winner overwrites them) when
// the LOCAL version loses.
pub(super) fn archived_from_payload(
    chunks: &[ChunkPayload],
) -> Vec<ArchivedChunk> {
    chunks
        .iter()
        .map(|c| ArchivedChunk {
            chunk_index: c.chunk_index,
            vector_b64: c.vector_b64.clone(),
            content_hash: c.content_hash.clone(),
            chunk_text: c.chunk_text.clone(),
            title: c.title.clone(),
            tags: c.tags.clone(),
            created_at: c.created_at.clone(),
        })
        .collect()
}

pub(super) fn archived_from_local(
    conn: &Connection,
    entity_id: &str,
    kind: &str,
) -> Result<Vec<ArchivedChunk>, SyncError> {
    Ok(archived_from_payload(&load_chunks(conn, entity_id, kind)?))
}
