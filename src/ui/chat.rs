use crate::services::audio::RecordingState;
use crate::services::rag;
use crate::ui::chat_input::ChatInputBar;
use crate::ui::icons::*;
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;

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

fn send_question(
    question: String,
    messages: &mut Signal<Vec<ChatMsg>>,
    loading: &mut Signal<bool>,
) {
    messages.write().push(ChatMsg::User(question.clone()));
    loading.set(true);
    let mut msgs = *messages;
    let mut ld = *loading;
    spawn(async move {
        match rag::query(&question).await {
            Ok(r) => {
                let sources = r
                    .sources
                    .iter()
                    .map(|s| ChatSource {
                        note_id: s.note_id.clone(),
                        title: s.title.clone(),
                        chunk_text: s.chunk_text.clone(),
                        distance: s.distance,
                    })
                    .collect();
                msgs.write().push(ChatMsg::Bot {
                    text: r.answer,
                    sources,
                });
            }
            Err(e) => {
                msgs.write().push(ChatMsg::Bot {
                    text: format!("Erreur : {e}"),
                    sources: vec![],
                });
            }
        }
        ld.set(false);
    });
}

#[component]
pub fn ChatView() -> Element {
    let mut app: AppState = use_context();
    let mut messages: Signal<Vec<ChatMsg>> = use_signal(Vec::new);
    let mut input = use_signal(String::new);
    let mut loading = use_signal(|| false);

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

    rsx! {
        div {
            class: "overflow-hidden pb-20",
            style: "height: calc(100% - var(--keyboard-inset, 0px));",
            div { id: "chat-messages", class: "h-full overflow-y-auto px-4 pt-4 pb-4",
                if is_empty && !loading() {
                    div {
                        class: "flex flex-col items-center justify-center px-6 h-full",
                        div { class: "text-gray-800 mb-5",
                            IconHeadCircuit { size: 80 }
                        }
                        p { class: "text-gray-900 font-semibold text-base mb-1",
                            "Chat avec tes notes"
                        }
                        p { class: "text-gray-400 text-sm text-center",
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
                                        div { class: "bg-ios-blue text-white rounded-2xl rounded-br-md px-4 py-2.5 max-w-[80%] text-sm leading-relaxed break-words",
                                            "{text}"
                                        }
                                    }
                                },
                                ChatMsg::Bot { text, sources } => rsx! {
                                    div {
                                        key: "{i}",
                                        class: "flex justify-start",
                                        style: "animation: fadeInUp 0.15s ease-out;",
                                        div { class: "bg-white rounded-2xl rounded-bl-md px-4 py-2.5 max-w-[85%] shadow-sm",
                                            div {
                                                class: "text-sm text-gray-900 leading-relaxed break-words prose prose-sm",
                                                dangerous_inner_html: md_to_html(text),
                                            }
                                            if !sources.is_empty() {
                                                div { class: "mt-2 pt-2 border-t border-gray-100 flex flex-col gap-1.5",
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
                                                        rsx! {
                                                            button {
                                                                key: "{j}",
                                                                class: "w-full text-left px-2.5 py-1.5 rounded-lg bg-ios-blue/5 active:bg-ios-blue/12",
                                                                onclick: move |_| app.view.set(View::NoteDetail { note_id: nid.clone() }),
                                                                div { class: "flex items-center justify-between",
                                                                    span { class: "text-[11px] font-medium text-ios-blue", "{src.title}" }
                                                                    span { class: "text-[10px] text-gray-400", "{pct}%" }
                                                                }
                                                                p { class: "text-[10px] text-gray-400 leading-tight mt-0.5 line-clamp-1", "{preview}" }
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
                                div { class: "bg-white rounded-2xl rounded-bl-md px-5 py-3.5 shadow-sm",
                                    div { class: "flex items-center gap-1.5",
                                        span {
                                            class: "w-1.5 h-1.5 rounded-full bg-gray-400",
                                            style: "animation: typingDot 1.2s ease-in-out infinite;",
                                        }
                                        span {
                                            class: "w-1.5 h-1.5 rounded-full bg-gray-400",
                                            style: "animation: typingDot 1.2s ease-in-out 0.12s infinite;",
                                        }
                                        span {
                                            class: "w-1.5 h-1.5 rounded-full bg-gray-400",
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
        ChatInputBar {
            input: input,
            disabled: loading(),
            on_send: move |q: String| {
                send_question(q, &mut messages, &mut loading);
            },
        }
    }
}
