use crate::db::Database;
use crate::services::audio::RecordingState;
use crate::services::rag;
use crate::ui::chat_input::ChatInputBar;
use crate::ui::icons::*;
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

fn md_to_html(md: &str) -> String {
    use pulldown_cmark::{html, Parser};
    let parser = Parser::new(md);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[derive(Clone)]
struct ChatSource {
    note_id: String,
    title: String,
    chunk_text: String,
    distance: f32,
}

#[derive(Clone)]
enum ChatMsg {
    User(String),
    Bot {
        text: String,
        sources: Vec<ChatSource>,
    },
}

fn tool_label(name: &str) -> &str {
    match name {
        "search_notes" => "Recherche dans les notes...",
        "create_note" => "Création de la note...",
        "summarize_folder" => "Résumé du dossier...",
        _ => "L'agent travaille...",
    }
}

fn send_question(
    question: String,
    messages: &mut Signal<Vec<ChatMsg>>,
    loading: &mut Signal<bool>,
    tool_status: &mut Signal<Option<String>>,
    conversation_id: Signal<Option<String>>,
    db: Signal<Arc<Database>>,
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

    spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                crate::services::tools::ToolEvent::Started(name) => {
                    ts.set(Some(tool_label(&name).to_string()));
                }
                crate::services::tools::ToolEvent::Finished(_) => {
                    ts.set(None);
                }
            }
        }
    });

    spawn(async move {
        match rag::query(&question, Some(tx)).await {
            Ok(r) => {
                let sources: Vec<ChatSource> = r
                    .sources
                    .iter()
                    .map(|s| ChatSource {
                        note_id: s.note_id.clone(),
                        title: s.title.clone(),
                        chunk_text: s.chunk_text.clone(),
                        distance: s.distance,
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
                let err_msg = format!("Erreur : {e}");
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

fn load_messages_from_db(db: &Database, conversation_id: &str) -> Vec<ChatMsg> {
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

#[component]
pub fn ChatView() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();

    let initial_conv_id = match (app.view)() {
        View::Chat { conversation_id } => conversation_id,
        _ => None,
    };

    let mut conversation_id: Signal<Option<String>> =
        use_signal(|| initial_conv_id.clone());

    let initial_msgs = if let Some(ref cid) = initial_conv_id {
        load_messages_from_db(&db(), cid)
    } else {
        vec![]
    };

    let mut messages: Signal<Vec<ChatMsg>> = use_signal(|| initial_msgs);
    let mut input = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut tool_status: Signal<Option<String>> = use_signal(|| None);
    let mut renaming = use_signal(|| false);
    let mut rename_input = use_signal(String::new);
    let mut confirm_delete = use_signal(|| false);

    use_effect(move || {
        if let RecordingState::Transcribed(text) = (app.recording_state)() {
            let current = input();
            if current.is_empty() {
                input.set(text);
            } else {
                input.set(format!("{} {}", current, text));
            }
            app.recording_state.set(RecordingState::Idle);
        }
    });

    use_effect(move || {
        let _len = messages().len();
        dioxus::document::eval(
            r#"
            let el = document.getElementById('chat-messages');
            if (el) el.scrollTop = el.scrollHeight;
        "#,
        );
    });

    let is_empty = messages().is_empty();
    let show_menu = (app.show_chat_menu)();
    let has_conversation = conversation_id().is_some();

    rsx! {
        if show_menu {
            div {
                class: "fixed inset-0 z-40",
                onclick: move |_| {
                    app.show_chat_menu.set(false);
                    renaming.set(false);
                },
            }
            div {
                class: "absolute right-4 top-1 z-50 bg-warm-white rounded-xl shadow-lg border border-stone-100 py-1 min-w-[220px]",
                if renaming() {
                    div { class: "px-4 py-3",
                        p { class: "text-[10px] font-medium text-stone-400 uppercase tracking-wide mb-2", "Renommer" }
                        input {
                            class: "w-full text-sm border border-stone-200 rounded-lg px-3 py-2 outline-none focus:border-ios-orange-dark",
                            value: "{rename_input}",
                            oninput: move |evt| rename_input.set(evt.value()),
                            onkeypress: move |evt| {
                                if evt.key() == Key::Enter {
                                    let v = rename_input().trim().to_string();
                                    if !v.is_empty() {
                                        if let Some(ref cid) = conversation_id() {
                                            let _ = db().update_conversation_title(cid, &v);
                                        }
                                    }
                                    renaming.set(false);
                                    app.show_chat_menu.set(false);
                                }
                            },
                        }
                        div { class: "flex gap-2 mt-2",
                            button {
                                class: "flex-1 h-8 text-sm text-stone-600 bg-stone-100 rounded-lg active:bg-stone-200",
                                onclick: move |_| {
                                    renaming.set(false);
                                    app.show_chat_menu.set(false);
                                },
                                "Annuler"
                            }
                            button {
                                class: "flex-1 h-8 text-sm text-white bg-ios-orange rounded-lg active:opacity-80",
                                onclick: move |_| {
                                    let v = rename_input().trim().to_string();
                                    if !v.is_empty() {
                                        if let Some(ref cid) = conversation_id() {
                                            let _ = db().update_conversation_title(cid, &v);
                                        }
                                    }
                                    renaming.set(false);
                                    app.show_chat_menu.set(false);
                                },
                                "OK"
                            }
                        }
                    }
                } else if confirm_delete() {
                    div { class: "px-4 py-3",
                        p { class: "text-[10px] font-medium text-stone-400 uppercase tracking-wide mb-1", "Supprimer ce chat ?" }
                        p { class: "text-xs text-stone-500 mb-3", "Cette action est irréversible." }
                        div { class: "flex gap-2",
                            button {
                                class: "flex-1 h-9 text-sm font-medium text-stone-900 bg-stone-100 rounded-full active:bg-stone-200",
                                onclick: move |_| {
                                    confirm_delete.set(false);
                                    app.show_chat_menu.set(false);
                                },
                                "Annuler"
                            }
                            button {
                                class: "flex-1 h-9 text-sm font-medium text-white bg-ios-red rounded-full active:opacity-80",
                                onclick: move |_| {
                                    if let Some(ref cid) = conversation_id() {
                                        let _ = db().delete_conversation(cid);
                                    }
                                    confirm_delete.set(false);
                                    app.show_chat_menu.set(false);
                                    app.sliding_out.set(true);
                                    spawn(async move {
                                        futures_timer::Delay::new(std::time::Duration::from_millis(150)).await;
                                        app.sliding_out.set(false);
                                        app.view.set(View::NotesList);
                                    });
                                },
                                "Supprimer"
                            }
                        }
                    }
                } else {
                    button {
                        class: "w-full flex items-center gap-3 px-4 py-3 text-sm text-stone-700 active:bg-stone-50",
                        disabled: !has_conversation,
                        onclick: move |_| {
                            if let Some(ref cid) = conversation_id() {
                                if let Ok(convs) = db().list_conversations() {
                                    if let Some(conv) = convs.iter().find(|c| &c.id == cid) {
                                        rename_input.set(conv.title.clone());
                                    }
                                }
                            }
                            renaming.set(true);
                        },
                        IconPencil { size: 18 }
                        "Renommer"
                    }
                    button {
                        class: "w-full flex items-center gap-3 px-4 py-3 text-sm text-ios-red active:bg-stone-50",
                        disabled: !has_conversation,
                        onclick: move |_| {
                            confirm_delete.set(true);
                        },
                        IconTrash { size: 18 }
                        "Supprimer le chat"
                    }
                }
            }
        }
        div {
            class: "overflow-hidden",
            style: "height: calc(100% - var(--keyboard-inset, 0px));",
            div { id: "chat-messages", class: "h-full overflow-y-auto px-4 pt-4 pb-40",
                if is_empty && !loading() {
                    div {
                        class: "flex flex-col items-center justify-center px-6 h-full",
                        img { src: asset!("/assets/flowflow-icon-300.png"), width: "150", height: "150", class: "mb-6 rounded-3xl" }
                        p { class: "text-stone-900 font-semibold text-base mb-1",
                            "Chat avec tes notes"
                        }
                        p { class: "text-stone-400 text-sm text-center",
                            "Pose une question, je cherche dans tes notes."
                        }
                    }
                } else {
                    div { class: "space-y-3",
                        {messages().iter().enumerate().map(|(i, msg)| {
                            match msg {
                                ChatMsg::User(text) => rsx! {
                                    div {
                                        key: "{i}",
                                        class: "flex justify-end",
                                        style: "animation: fadeInUp 0.15s ease-out;",
                                        div { class: "bg-ios-orange text-white rounded-2xl rounded-br-md px-4 py-2.5 max-w-[80%] text-sm leading-relaxed break-words",
                                            "{text}"
                                        }
                                    }
                                },
                                ChatMsg::Bot { text, sources } => rsx! {
                                    div {
                                        key: "{i}",
                                        class: "flex justify-start",
                                        style: "animation: fadeInUp 0.15s ease-out;",
                                        div { class: "bg-warm-white border border-ios-orange/10 rounded-2xl rounded-bl-md px-4 py-2.5 max-w-[85%] shadow-sm",
                                            div {
                                                class: "text-sm text-stone-900 leading-relaxed break-words prose prose-sm",
                                                dangerous_inner_html: md_to_html(text),
                                            }
                                            if !sources.is_empty() {
                                                div { class: "mt-2 pt-2 border-t border-stone-100 flex flex-col gap-1.5",
                                                    {sources.iter().enumerate().map(|(j, src)| {
                                                        let nid = src.note_id.clone();
                                                        let preview = if src.chunk_text.chars().count() > 80 {
                                                            let s: String = src.chunk_text.chars().take(80).collect();
                                                            format!("{s}...")
                                                        } else {
                                                            src.chunk_text.clone()
                                                        };
                                                        let pct = ((1.0 - src.distance) * 100.0).round() as u32;
                                                        let mut app = app.clone();
                                                        let conv_for_back = conversation_id();
                                                        rsx! {
                                                            button {
                                                                key: "{j}",
                                                                class: "w-full text-left px-2.5 py-1.5 rounded-lg bg-warm-white border border-ios-orange/20 active:bg-ios-orange-50",
                                                                onclick: move |_| {
                                                                    app.previous_view.set(Some(View::Chat { conversation_id: conv_for_back.clone() }));
                                                                    app.view.set(View::NoteDetail { note_id: nid.clone() });
                                                                },
                                                                div { class: "flex items-center justify-between",
                                                                    span { class: "text-[11px] font-medium text-ios-orange-dark", "{src.title}" }
                                                                    span { class: "text-[10px] text-stone-400", "{pct}%" }
                                                                }
                                                                p { class: "text-[10px] text-stone-400 leading-tight mt-0.5 line-clamp-1", "{preview}" }
                                                            }
                                                        }
                                                    })}
                                                }
                                            }
                                        }
                                    }
                                },
                            }
                        })}
                        if loading() {
                            div {
                                class: "flex justify-start",
                                style: "animation: fadeInUp 0.15s ease-out;",
                                div { class: "bg-warm-white border border-ios-orange/10 rounded-2xl rounded-bl-md px-5 py-3.5 shadow-sm",
                                    if let Some(ref status) = tool_status() {
                                        div { class: "flex items-center gap-2",
                                            span {
                                                class: "w-1.5 h-1.5 rounded-full bg-ios-orange",
                                                style: "animation: pulseSoft 1.2s ease-in-out infinite;",
                                            }
                                            span { class: "text-xs text-stone-500", "{status}" }
                                        }
                                    } else {
                                        div { class: "flex items-center gap-1.5",
                                            span {
                                                class: "w-1.5 h-1.5 rounded-full bg-ios-orange/60",
                                                style: "animation: typingDot 1.2s ease-in-out infinite;",
                                            }
                                            span {
                                                class: "w-1.5 h-1.5 rounded-full bg-ios-orange/60",
                                                style: "animation: typingDot 1.2s ease-in-out 0.12s infinite;",
                                            }
                                            span {
                                                class: "w-1.5 h-1.5 rounded-full bg-ios-orange/60",
                                                style: "animation: typingDot 1.2s ease-in-out 0.24s infinite;",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        ChatInputBar {
            input: input,
            disabled: loading(),
            on_send: move |q: String| {
                if conversation_id().is_none() {
                    let title: String = q.chars().take(50).collect();
                    if let Ok(conv) = db().create_conversation(&title) {
                        let cid = conv.id.clone();
                        conversation_id.set(Some(cid));
                    }
                }
                send_question(q, &mut messages, &mut loading, &mut tool_status, conversation_id, db);
            },
        }
    }
}
