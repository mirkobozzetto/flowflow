use crate::db::chunk_repo::{content_hash, ChunkRecord};
use crate::services::ai::chunk_text;
use crate::services::llm::LlmClient;
use crate::services::vectordb::{Chunk, VectorStore};

const EMBED_MIN_CHARS: usize = 10;

pub(crate) fn too_short_to_embed(content: &str) -> bool {
    content.trim().chars().count() < EMBED_MIN_CHARS
}

fn embed_text(title: &str, content: &str) -> String {
    let t = title.trim();
    if t.is_empty() {
        content.to_string()
    } else {
        format!("{t}\n{content}")
    }
}

// Chunks have no tracking triggers (they travel inside their owner's sync
// payload, RFC 0004 T17), so after writing fresh BLOBs the owner's sync meta
// must be bumped for the new vectors to propagate. The alive-check and the
// bump run in ONE IMMEDIATE transaction: the write lock is taken before the
// check, so a delete committing on another connection can never slip between
// them (an embed finishing after a delete would otherwise resurrect the
// tombstone on peers), and the seq allocation inside the bump stays atomic
// across connections. A successful bump pokes the sync engine: the embed
// thread finishes AFTER the save-triggered push, so without its own trigger
// the fresh vectors would sit stranded until an unrelated event (and the
// earlier chunkless push would have wiped the peer's previous vectors).
pub(crate) fn bump_owner_meta_if_alive(
    db: &crate::db::Database,
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
        crate::db::sync_meta::mark_entity_updated(&conn, owner_kind, owner_id)
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
                crate::services::sync::engine::poke_after_data_change();
            }
        }
        Err(e) => {
            log(&format!("chunk meta bump: {e}"));
            let _ = conn.execute_batch("ROLLBACK;");
        }
    }
}

fn persist_chunk_blobs(owner_id: &str, owner_kind: &str, entries: &[Chunk]) {
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
    match crate::db::Database::open() {
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

async fn purge_owner_chunks(owner_id: &str, owner_kind: &str) {
    let mut had_blobs = false;
    if let Ok(db) = crate::db::Database::open() {
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

fn ai_consent_granted() -> bool {
    match crate::db::Database::open() {
        Ok(db) => db.get_setting("ai_consent") == Some("true".to_string()),
        Err(_) => false,
    }
}

fn log(msg: &str) {
    eprintln!("{msg}");
    #[cfg(target_os = "ios")]
    {
        use std::ffi::CString;
        extern "C" {
            fn syslog(priority: i32, format: *const std::ffi::c_char, ...);
        }
        if let Ok(cmsg) = CString::new(format!("[FlowFlow] {msg}")) {
            unsafe {
                syslog(
                    4,
                    b"%s\0".as_ptr() as *const std::ffi::c_char,
                    cmsg.as_ptr(),
                );
            }
        }
    }
}

pub(crate) async fn embed_note_core(
    store: &VectorStore,
    ai: &LlmClient,
    note_id: &str,
    title: &str,
    content: &str,
    tags: &[String],
    note_created_at: &str,
) -> Result<usize, String> {
    let chunks_text = chunk_text(&embed_text(title, content));
    let tags_json =
        serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());
    let mut entries = Vec::new();
    for (i, text) in chunks_text.iter().enumerate() {
        let vector = ai
            .embed(text)
            .await
            .map_err(|e| format!("embed chunk {i}: {e}"))?;
        entries.push(Chunk {
            id: format!("note:{note_id}:{i}"),
            note_id: note_id.to_string(),
            chunk_text: text.clone(),
            chunk_index: i as i32,
            vector,
            title: title.to_string(),
            tags: tags_json.clone(),
            created_at: note_created_at.to_string(),
        });
    }
    let n = entries.len();
    persist_chunk_blobs(note_id, "note", &entries);
    store
        .store_chunks(entries)
        .await
        .map_err(|e| format!("embed store: {e}"))?;
    Ok(n)
}

pub fn embed_note(
    note_id: String,
    title: String,
    content: String,
    tags: Vec<String>,
    note_created_at: String,
) {
    log(&format!(
        "embed triggered for {note_id} ({} chars)",
        content.chars().count()
    ));
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if too_short_to_embed(&content) {
                log("embed skip: too short");
                purge_owner_chunks(&note_id, "note").await;
                return;
            }
            if !ai_consent_granted() {
                log("embed skip: ai_consent not granted");
                return;
            }
            let ai = match LlmClient::from_env() {
                Ok(c) => c,
                Err(e) => {
                    log(&format!("embed skip: {e}"));
                    return;
                }
            };
            let store = match VectorStore::open().await {
                Ok(s) => s,
                Err(e) => {
                    log(&format!("embed store error: {e}"));
                    return;
                }
            };
            match embed_note_core(
                &store,
                &ai,
                &note_id,
                &title,
                &content,
                &tags,
                &note_created_at,
            )
            .await
            {
                Ok(n) => log(&format!("embed done for {note_id} ({n} chunks)")),
                Err(e) => log(&format!("embed {note_id}: {e}")),
            }
        });
    });
}

pub(crate) async fn embed_missing_notes(
    db: &crate::db::Database,
    store: &VectorStore,
) -> usize {
    if !ai_consent_granted() {
        return 0;
    }
    let ai = match LlmClient::from_env() {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let notes = match db.list_notes() {
        Ok(n) => n,
        Err(e) => {
            log(&format!("embed missing: list notes {e}"));
            return 0;
        }
    };
    let mut embedded = 0usize;
    for note in &notes {
        if too_short_to_embed(&note.content) {
            continue;
        }
        match db.count_chunks_for_owner(&note.id, "note") {
            Ok(0) => {}
            Ok(_) => continue,
            Err(e) => {
                log(&format!("embed missing: count {} {e}", note.id));
                continue;
            }
        }
        let title = note.title.clone().unwrap_or_default();
        match embed_note_core(
            store,
            &ai,
            &note.id,
            &title,
            &note.content,
            &note.tags,
            &note.created_at,
        )
        .await
        {
            Ok(n) => {
                embedded += 1;
                log(&format!("embed missing: {} (+{n} chunks)", note.id));
            }
            Err(e) => log(&format!("embed missing: {} {e}", note.id)),
        }
    }
    if embedded > 0 {
        log(&format!("embed missing: {embedded} notes embedded"));
    }
    embedded
}

pub fn delete_note_embeddings(note_id: String) {
    log(&format!("embed delete for {note_id}"));
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            match VectorStore::open().await {
                Ok(store) => {
                    let _ = store.delete_note_chunks(&note_id).await;
                    log(&format!("embed deleted {note_id}"));
                }
                Err(e) => log(&format!("embed delete error: {e}")),
            }
        });
    });
}

pub fn embed_attachment(
    attachment_id: String,
    parent_note_id: String,
    filename: String,
    content: String,
) {
    log(&format!(
        "embed attachment {attachment_id} ({} chars)",
        content.chars().count()
    ));
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if too_short_to_embed(&content) {
                log("embed attachment skip: too short");
                purge_owner_chunks(&attachment_id, "attachment").await;
                return;
            }
            if !ai_consent_granted() {
                log("embed attachment skip: ai_consent not granted");
                return;
            }
            let ai = match LlmClient::from_env() {
                Ok(c) => c,
                Err(e) => {
                    log(&format!("embed attachment skip: {e}"));
                    return;
                }
            };
            let store = match VectorStore::open().await {
                Ok(s) => s,
                Err(e) => {
                    log(&format!("embed attachment store error: {e}"));
                    return;
                }
            };
            let chunks_text = chunk_text(&content);
            log(&format!("embed attachment: {} chunks", chunks_text.len()));
            let now = crate::db::now_iso();
            let mut entries = Vec::new();
            for (i, text) in chunks_text.iter().enumerate() {
                match ai.embed(text).await {
                    Ok(vector) => {
                        entries.push(Chunk {
                            id: format!("att:{attachment_id}:{i}"),
                            note_id: parent_note_id.clone(),
                            chunk_text: text.clone(),
                            chunk_index: i as i32,
                            vector,
                            title: filename.clone(),
                            tags: "[]".to_string(),
                            created_at: now.clone(),
                        });
                    }
                    Err(e) => {
                        log(&format!("embed attachment chunk {i}: {e}"));
                        return;
                    }
                }
            }
            persist_chunk_blobs(&attachment_id, "attachment", &entries);
            match store.store_chunks(entries).await {
                Ok(()) => {
                    log(&format!("embed attachment done for {attachment_id}"))
                }
                Err(e) => log(&format!("embed attachment store: {e}")),
            }
        });
    });
}

pub fn delete_attachment_embeddings(attachment_id: String) {
    log(&format!("embed delete attachment {attachment_id}"));
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            match VectorStore::open().await {
                Ok(store) => {
                    let _ =
                        store.delete_attachment_chunks(&attachment_id).await;
                    log(&format!("embed deleted attachment {attachment_id}"));
                }
                Err(e) => log(&format!("embed delete attachment error: {e}")),
            }
        });
    });
}
