use crate::application::ai::chunk_text;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::vectordb::{Chunk, VectorStore};

mod chunk_store;

pub(crate) use chunk_store::bump_owner_meta_if_alive;
use chunk_store::{persist_chunk_blobs, purge_owner_chunks};

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

fn ai_consent_granted() -> bool {
    match crate::infrastructure::persistence::Database::open() {
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
    if let Ok(db) = crate::infrastructure::persistence::Database::open() {
        crate::application::related::compute_note_links(
            store, ai, &db, note_id,
        )
        .await;
    }
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
                // Stale active links would point at content that no longer exists.
                if let Ok(db) =
                    crate::infrastructure::persistence::Database::open()
                {
                    let _ = db.replace_note_links(&note_id, &[]);
                }
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
    db: &crate::infrastructure::persistence::Database,
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
            let now = crate::infrastructure::persistence::now_iso();
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
