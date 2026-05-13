use crate::db::Database;
use crate::services::audio::RecordingState;
use crate::ui::chat::actions::{
    load_messages_from_db, md_to_html, send_question,
};
use crate::ui::chat::menu::ChatMenu;
use crate::ui::chat::models::ChatMsg;
use crate::ui::chat_input::ChatInputBar;
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

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
    let renaming = use_signal(|| false);
    let rename_input = use_signal(String::new);
    let confirm_delete = use_signal(|| false);

    use_effect(move || {
        if let RecordingState::Transcribed(text) = (app.recording_state)() {
            let current = input();
            if current.is_empty() {
                input.set(text);
            } else {
                input.set(format!("{} {}", current, text));
            }
            app.recording_state.set(RecordingState::Idle);
            dioxus::document::eval(
                r#"
                requestAnimationFrame(() => {
                    var ta = document.querySelector('.chat-textarea');
                    if (ta) { ta.style.height = 'auto'; ta.style.height = ta.scrollHeight + 'px'; }
                });
                "#,
            );
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

    rsx! {
        if show_menu {
            ChatMenu {
                conversation_id: conversation_id,
                renaming: renaming,
                rename_input: rename_input,
                confirm_delete: confirm_delete,
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
                                                        let mut app = app;
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
