use crate::services::ai::chunk_text;
use crate::services::llm::LlmClient;
use crate::services::vectordb::{Chunk, VectorStore};

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

pub fn embed_note(note_id: String, title: String, content: String) {
    log(&format!(
        "embed triggered for {note_id} ({} chars)",
        content.len()
    ));
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if content.len() < 50 {
                log("embed skip: too short");
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
            let now = crate::db::now_iso();
            let mut entries = Vec::new();
            for (i, text) in chunks_text.iter().enumerate() {
                match ai.embed(text).await {
                    Ok(vector) => {
                        log(&format!(
                            "embed chunk {i} OK ({} dims)",
                            vector.len()
                        ));
                        entries.push(Chunk {
                            id: uuid::Uuid::new_v4().to_string(),
                            note_id: note_id.clone(),
                            chunk_text: text.clone(),
                            chunk_index: i as i32,
                            vector,
                            title: title.clone(),
                            tags: "[]".to_string(),
                            created_at: now.clone(),
                        });
                    }
                    Err(e) => {
                        log(&format!("embed chunk {i}: {e}"));
                        return;
                    }
                }
            }
            let _ = store.delete_note_chunks(&note_id).await;
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
            let _ = store.delete_attachment_chunks(&attachment_id).await;
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
