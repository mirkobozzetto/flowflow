use crate::db::Database;
use crate::services::rag;
use crate::ui::chat::models::{tool_label, ChatMsg, ChatSource};
use dioxus::prelude::*;
use std::sync::Arc;

pub fn md_to_html(md: &str) -> String {
    use pulldown_cmark::{html, Parser};
    let parser = Parser::new(md);
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
    folder_id: Option<String>,
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
                crate::services::tools::ToolEvent::Started(name) => {
                    ts.set(Some(tool_label(&lang_for_tools, &name)));
                }
                crate::services::tools::ToolEvent::Finished(_) => {
                    ts.set(None);
                }
            }
        }
    });

    spawn(async move {
        match rag::query(&question, Some(tx), folder_id).await {
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
                    })
                    .collect();

                let sources_json = if sources.is_empty() {
                    None
                } else {
                    let src_data: Vec<serde_json::Value> = sources
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "note_id": s.note_id,
                                "title": s.title,
                                "chunk_text": s.chunk_text,
                                "distance": s.distance,
                                "created_at": s.created_at,
                            })
                        })
                        .collect();
                    Some(serde_json::to_string(&src_data).unwrap_or_default())
                };

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
                let err_msg = format!(
                    "{} : {}",
                    crate::services::i18n::t(&lang, "chat-error"),
                    e
                );
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
                    .and_then(|json| {
                        serde_json::from_str::<Vec<serde_json::Value>>(json)
                            .ok()
                    })
                    .map(|arr| {
                        arr.into_iter()
                            .map(|v| ChatSource {
                                note_id: v["note_id"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                title: v["title"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                chunk_text: v["chunk_text"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                distance: v["distance"].as_f64().unwrap_or(0.0)
                                    as f32,
                                created_at: v["created_at"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                ChatMsg::Bot {
                    text: m.content,
                    sources,
                }
            }
        })
        .collect()
}
