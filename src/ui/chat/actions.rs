use crate::application::rag;
use crate::domain::{generate_auto_title, ChatScope, NewTextNote, UpdateNote};
use crate::infrastructure::persistence::Database;
use crate::ui::chat::models::{tool_label, ChatMsg, ChatSource};
use dioxus::prelude::*;
use std::sync::Arc;

pub fn serialize_sources(sources: &[ChatSource]) -> Option<String> {
    if sources.is_empty() {
        return None;
    }
    let data: Vec<serde_json::Value> = sources
        .iter()
        .map(|s| {
            serde_json::json!({
                "note_id": s.note_id,
                "title": s.title,
                "chunk_text": s.chunk_text,
                "distance": s.distance,
                "created_at": s.created_at,
                "url": s.url,
            })
        })
        .collect();
    Some(serde_json::to_string(&data).unwrap_or_default())
}

pub fn parse_sources(json: &str) -> Vec<ChatSource> {
    serde_json::from_str::<Vec<serde_json::Value>>(json)
        .ok()
        .map(|arr| {
            arr.into_iter()
                .map(|v| ChatSource {
                    note_id: v["note_id"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    title: v["title"].as_str().unwrap_or_default().to_string(),
                    chunk_text: v["chunk_text"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    distance: v["distance"].as_f64().unwrap_or(0.0) as f32,
                    created_at: v["created_at"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    url: v["url"].as_str().map(String::from),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn save_as_note(
    db: &Database,
    folder_id: Option<&str>,
    content: String,
    tags: Vec<String>,
    web_sources: &[ChatSource],
    lang: &str,
) -> Result<String, String> {
    if let Some(existing) = find_saved_note(db, folder_id, &content) {
        return Ok(existing);
    }
    let note = db.create_text_note(&NewTextNote {
        title: Some(generate_auto_title(lang)),
        content,
        tags,
    })?;
    if let Some(fid) = folder_id {
        let _ = db.add_note_to_folder(&note.id, fid);
    }
    if let Some(json) = serialize_sources(web_sources) {
        let _ = db.set_note_sources(&note.id, Some(&json));
    }
    let title_for_embed = note.title.clone().unwrap_or_default();
    crate::application::embed::embed_note(
        note.id.clone(),
        title_for_embed,
        note.content.clone(),
        note.tags.clone(),
        note.created_at.clone(),
    );
    generate_note_title_bg(
        note.id.clone(),
        note.content.clone(),
        lang.to_string(),
    );
    Ok(note.id)
}

pub fn generate_note_title_bg(note_id: String, content: String, lang: String) {
    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Runtime::new() else {
            return;
        };
        rt.block_on(async move {
            let Ok(db) = Database::open() else {
                return;
            };
            let Ok(ai) = crate::infrastructure::llm::LlmClient::from_db(&db)
            else {
                return;
            };
            if let Ok(title) = ai.generate_title(&content, &lang).await {
                let _ = db.update_note(
                    &note_id,
                    &UpdateNote {
                        title: Some(title),
                        content: None,
                        tags: None,
                    },
                );
            }
        });
    });
}

pub fn scope_save_folder(
    db: &Database,
    scope: &Option<ChatScope>,
) -> Option<String> {
    match scope {
        Some(ChatScope::Folder(id)) => Some(id.clone()),
        Some(ChatScope::Thread(tid)) => {
            db.get_thread(tid).ok().flatten().and_then(|t| t.folder_id)
        }
        None => None,
    }
}

pub fn find_saved_note(
    db: &Database,
    folder_id: Option<&str>,
    content: &str,
) -> Option<String> {
    let notes = match folder_id {
        Some(fid) => db.list_notes_in_folder(fid).ok()?,
        None => db.list_notes().ok()?,
    };
    notes
        .into_iter()
        .find(|n| n.content == content)
        .map(|n| n.id)
}

pub fn md_to_html(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    // GFM tables/strikethrough/tasklists are off by default in pulldown-cmark, so a `| a | b |`
    // table from the agent rendered as raw piped text. Enable them.
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, opts);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[allow(clippy::too_many_arguments)]
pub fn send_question(
    question: String,
    messages: &mut Signal<Vec<ChatMsg>>,
    loading: &mut Signal<bool>,
    tool_status: &mut Signal<Option<String>>,
    conversation_id: Signal<Option<String>>,
    db: Signal<Arc<Database>>,
    scope: Option<rag::ChatScope>,
    lang: String,
) {
    messages.write().push(ChatMsg::User(question.clone()));
    loading.set(true);
    tool_status.set(None);

    let conv_id = conversation_id();
    if let Some(ref cid) = conv_id {
        let _ = db().add_message(cid, "user", &question, None);
        let _ = db().touch_conversation(cid);
    }

    let mut msgs = *messages;
    let mut ld = *loading;
    let mut ts = *tool_status;
    let conv_signal = conversation_id;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let lang_for_tools = lang.clone();
    spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                crate::application::tools::ToolEvent::Started(name) => {
                    ts.set(Some(tool_label(&lang_for_tools, &name)));
                }
                crate::application::tools::ToolEvent::Finished(_) => {
                    ts.set(None);
                }
            }
        }
    });

    let lang_for_query = lang.clone();
    spawn(async move {
        // "lance xxx" runs the note-action path (clean confirmation + link card); anything else
        // is a normal RAG question. Narrow trigger so real questions are never hijacked.
        let result = if crate::application::intent::is_action_trigger(&question)
        {
            rag::run_action(&question, Some(tx)).await
        } else {
            rag::query(&question, Some(tx), scope, &lang_for_query).await
        };
        match result {
            Ok(r) => {
                let sources: Vec<ChatSource> = r
                    .sources
                    .iter()
                    .map(|s| ChatSource {
                        note_id: s.note_id.clone(),
                        title: s.title.clone(),
                        chunk_text: s.chunk_text.clone(),
                        distance: s.distance,
                        created_at: s.created_at.clone(),
                        url: s.url.clone(),
                    })
                    .collect();

                let sources_json = serialize_sources(&sources);

                if let Some(ref cid) = conv_signal() {
                    let _ = db().add_message(
                        cid,
                        "bot",
                        &r.answer,
                        sources_json.as_deref(),
                    );
                }

                msgs.write().push(ChatMsg::Bot {
                    text: r.answer,
                    sources,
                });
            }
            Err(e) => {
                let err_msg = chat_error_message(&lang, &e);
                if let Some(ref cid) = conv_signal() {
                    let _ = db().add_message(cid, "bot", &err_msg, None);
                }
                msgs.write().push(ChatMsg::Bot {
                    text: err_msg,
                    sources: vec![],
                });
            }
        }
        ts.set(None);
        ld.set(false);
    });
}

// A connector/auth failure that reaches the chat gets an actionable hint pointing to the
// Connections screen; everything else stays the generic chat error. Heuristic on the
// (stringified) error since rag::query collapses all errors to String.
fn chat_error_message(lang: &str, err: &str) -> String {
    let low = err.to_lowercase();
    let connector = low.contains("unauthorized")
        || low.contains("connector")
        || low.contains("backend")
        || low.contains("mcp");
    let key = if connector {
        "chat-connector-error"
    } else {
        "chat-error"
    };
    format!("{} : {}", crate::application::i18n::t(lang, key), err)
}

pub fn load_messages_from_db(
    db: &Database,
    conversation_id: &str,
) -> Vec<ChatMsg> {
    let db_msgs = db.list_messages(conversation_id).unwrap_or_default();
    db_msgs
        .into_iter()
        .map(|m| {
            if m.role == "user" {
                ChatMsg::User(m.content)
            } else {
                let sources = m
                    .sources_json
                    .as_deref()
                    .map(parse_sources)
                    .unwrap_or_default();
                ChatMsg::Bot {
                    text: m.content,
                    sources,
                }
            }
        })
        .collect()
}
