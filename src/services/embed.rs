use crate::db::chunk_repo::{content_hash, ChunkRecord};
use crate::services::ai::chunk_text;
use crate::services::llm::LlmClient;
use crate::services::vectordb::{Chunk, VectorStore};

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

pub fn embed_note(
    note_id: String,
    title: String,
    content: String,
    tags: Vec<String>,
    note_created_at: String,
) {
    log(&format!(
        "embed triggered for {note_id} ({} chars)",
        content.len()
    ));
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if content.len() < 50 {
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
            let chunks_text = chunk_text(&content);
            log(&format!("embed: {} chunks", chunks_text.len()));
            let tags_json = serde_json::to_string(&tags)
                .unwrap_or_else(|_| "[]".to_string());
            let mut entries = Vec::new();
            for (i, text) in chunks_text.iter().enumerate() {
                match ai.embed(text).await {
                    Ok(vector) => {
                        log(&format!(
                            "embed chunk {i} OK ({} dims)",
                            vector.len()
                        ));
                        entries.push(Chunk {
                            id: format!("note:{note_id}:{i}"),
                            note_id: note_id.clone(),
                            chunk_text: text.clone(),
                            chunk_index: i as i32,
                            vector,
                            title: title.clone(),
                            tags: tags_json.clone(),
                            created_at: note_created_at.clone(),
                        });
                    }
                    Err(e) => {
                        log(&format!("embed chunk {i}: {e}"));
                        return;
                    }
                }
            }
            persist_chunk_blobs(&note_id, "note", &entries);
            match store.store_chunks(entries).await {
                Ok(()) => log(&format!("embed done for {note_id}")),
                Err(e) => log(&format!("embed store: {e}")),
            }
        });
    });
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
        content.len()
    ));
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if content.len() < 50 {
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
