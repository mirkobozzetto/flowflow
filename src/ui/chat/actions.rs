use crate::application::approvals::{
    is_renderable_role, reload_proposal, PersistedProposal, ProposalView,
};
use crate::application::rag;
use crate::application::tools::ToolEvent;
use crate::domain::{generate_auto_title, ChatScope, NewTextNote, UpdateNote};
use crate::infrastructure::persistence::Database;
use crate::ui::chat::models::{
    tool_label, ChatMsg, ChatSource, ProposalStatus,
};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

fn persist_status(status: &ProposalStatus) -> &'static str {
    match status {
        ProposalStatus::Pending => "pending",
        ProposalStatus::Approved => "approved",
        ProposalStatus::Edited => "edited",
        ProposalStatus::Rejected => "rejected",
        ProposalStatus::Expired => "expired",
    }
}

// Set an approval card's status by its view id (both the resolved event and a locally-detected
// expiry route through here); clears any stale edit error on the way.
pub fn update_proposal_status(
    messages: &mut Signal<Vec<ChatMsg>>,
    id: &str,
    status: ProposalStatus,
) {
    let mut w = messages.write();
    for m in w.iter_mut() {
        if let ChatMsg::Proposal {
            view,
            status: s,
            edit_error,
        } = m
        {
            if view.id == id {
                *s = status;
                *edit_error = None;
                return;
            }
        }
    }
}

fn set_proposal_edit_error(
    messages: &mut Signal<Vec<ChatMsg>>,
    id: &str,
    reason: String,
) {
    let mut w = messages.write();
    for m in w.iter_mut() {
        if let ChatMsg::Proposal {
            view, edit_error, ..
        } = m
        {
            if view.id == id {
                *edit_error = Some(reason);
                return;
            }
        }
    }
}

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
            if let Ok(title) = crate::application::titling::generate_title(
                &ai, &content, &lang,
            )
            .await
            {
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
    web: bool,
    mention_ids: Vec<String>,
    lang: String,
    app: AppState,
) {
    // Snapshot the recent turns BEFORE pushing the new question: this is the conversation
    // context rag::query rewrites follow-ups against ("et lui ?" -> the referent's name).
    let history: Vec<rag::ChatTurn> = {
        let msgs = messages.read();
        let skip = msgs
            .len()
            .saturating_sub(crate::application::constants::CHAT_HISTORY_TURNS);
        msgs.iter()
            .skip(skip)
            .filter_map(|m| match m {
                ChatMsg::User(t) => Some(rag::ChatTurn {
                    role: "user".into(),
                    content: t.clone(),
                }),
                ChatMsg::Bot { text, .. } => Some(rag::ChatTurn {
                    role: "bot".into(),
                    content: text.clone(),
                }),
                // A proposal turn is not conversational context; it never rewrites a follow-up.
                ChatMsg::Proposal { .. } => None,
            })
            .collect()
    };

    // Only the opening turn carries the placeholder title the LLM refines later.
    let first_turn = history.is_empty();
    let placeholder: String = question.chars().take(50).collect();

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
    let mut msgs_ev = *messages;
    let db_ev = db;
    let conv_ev = conversation_id;
    spawn(async move {
        // View id -> (persisted message id, its view), so a resolution can rewrite the exact
        // row this run wrote at propose time.
        let mut rows: std::collections::HashMap<
            String,
            (String, ProposalView),
        > = std::collections::HashMap::new();
        while let Some(event) = rx.recv().await {
            match event {
                ToolEvent::Started(name) => {
                    ts.set(Some(tool_label(&lang_for_tools, &name)));
                }
                ToolEvent::Finished(_) => {
                    ts.set(None);
                }
                ToolEvent::Proposal(view) => {
                    let id = view.id.clone();
                    // Persist the card at propose time so a run that dies mid-hold leaves an
                    // Expired-on-reload row instead of nothing.
                    if let Some(cid) = conv_ev() {
                        let content =
                            serde_json::to_string(&PersistedProposal {
                                view: view.clone(),
                                status: "pending".to_string(),
                            })
                            .unwrap_or_default();
                        if let Ok(msg) = db_ev()
                            .add_message(&cid, "proposal", &content, None)
                        {
                            rows.insert(id.clone(), (msg.id, view.clone()));
                        }
                    }
                    msgs_ev.write().push(ChatMsg::Proposal {
                        view,
                        status: ProposalStatus::Pending,
                        edit_error: None,
                    });
                }
                ToolEvent::ProposalResolved { id, status } => {
                    let ui_status = ProposalStatus::from(status);
                    update_proposal_status(
                        &mut msgs_ev,
                        &id,
                        ui_status.clone(),
                    );
                    if let Some((msg_id, view)) = rows.get(&id) {
                        let content =
                            serde_json::to_string(&PersistedProposal {
                                view: view.clone(),
                                status: persist_status(&ui_status).to_string(),
                            })
                            .unwrap_or_default();
                        let _ =
                            db_ev().update_message_content(msg_id, &content);
                    }
                }
                ToolEvent::EditRejected { id, reason } => {
                    set_proposal_edit_error(&mut msgs_ev, &id, reason);
                }
            }
        }
    });

    let lang_for_query = lang.clone();
    spawn(async move {
        // Resolution order (08-activation): an installed agent's alias or trigger words fire
        // that agent's chain; else "lance xxx" runs the generic note-action path; anything
        // else is a normal RAG question. No match never guesses an agent.
        let activation =
            crate::application::agent_activation::resolve(&db(), &question)
                .await;
        let result = if let Some(act) = activation {
            crate::application::agent_activation::run(
                &db(),
                &act,
                &question,
                tx,
            )
            .await
            .map(|(answer, trace)| rag::RagResponse {
                answer,
                sources: vec![],
                trace: Some(trace),
            })
        } else if crate::application::intent::is_action_trigger(&question) {
            rag::run_action(&question, Some(tx)).await
        } else {
            rag::query(
                &question,
                Some(tx),
                scope,
                web,
                mention_ids,
                history,
                &lang_for_query,
            )
            .await
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
                    if let Ok(msg) = db().add_message(
                        cid,
                        "bot",
                        &r.answer,
                        sources_json.as_deref(),
                    ) {
                        if let Some(ref trace) = r.trace {
                            let _ = db()
                                .set_message_trace(&msg.id, &trace.to_json());
                        }
                    }
                }

                let title_source =
                    first_turn.then(|| format!("{question}\n\n{}", r.answer));

                msgs.write().push(ChatMsg::Bot {
                    text: r.answer,
                    sources,
                    trace: r.trace,
                });

                if let (Some(source), Some(cid)) = (title_source, conv_signal())
                {
                    let titled =
                        crate::application::titling::retitle_conversation(
                            &db(),
                            &cid,
                            &placeholder,
                            &source,
                            &lang,
                        )
                        .await;
                    if titled.is_some() {
                        // The drawer stays mounted: the Chats list only re-reads on this bump.
                        let mut app = app;
                        app.invalidate_data();
                    }
                }
            }
            Err(e) => {
                let err_msg = chat_error_message(&lang, &e);
                if let Some(ref cid) = conv_signal() {
                    let _ = db().add_message(cid, "bot", &err_msg, None);
                }
                msgs.write().push(ChatMsg::Bot {
                    text: err_msg,
                    sources: vec![],
                    trace: None,
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
        .filter_map(|m| {
            if !is_renderable_role(&m.role) {
                return None;
            }
            match m.role.as_str() {
                "user" => Some(ChatMsg::User(m.content)),
                "bot" => {
                    let sources = m
                        .sources_json
                        .as_deref()
                        .map(parse_sources)
                        .unwrap_or_default();
                    let trace = db
                        .message_trace(&m.id)
                        .as_deref()
                        .and_then(crate::application::rag::RagTrace::from_json);
                    Some(ChatMsg::Bot {
                        text: m.content,
                        sources,
                        trace,
                    })
                }
                // Only "proposal" remains; a run never survives the app, so a persisted
                // Pending reloads as Expired (handled by reload_proposal).
                _ => reload_proposal(&m.content).map(|(view, status)| {
                    ChatMsg::Proposal {
                        view,
                        status: status.into(),
                        edit_error: None,
                    }
                }),
            }
        })
        .collect()
}
