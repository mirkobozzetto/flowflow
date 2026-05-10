use crate::services::audio::RecordingState;
use crate::services::rag;
use crate::ui::chat_input::ChatInputBar;
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;

#[derive(Clone)]
struct ChatSource {
    title: String,
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
                        title: s.title.clone(),
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

    let is_empty = messages().is_empty();

    rsx! {
        div { class: "pb-20",
            div { class: "overflow-y-auto px-4 py-4",
                style: "min-height: calc(100vh - 120px);",
                if is_empty && !loading() {
                    div {
                        class: "flex flex-col items-center justify-center px-6",
                        style: "min-height: calc(100vh - 200px);",
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
                                        div { class: "bg-ios-blue text-white rounded-2xl rounded-br-md px-4 py-2.5 max-w-[80%] text-sm leading-relaxed",
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
                                            p { class: "text-sm text-gray-900 leading-relaxed whitespace-pre-wrap",
                                                "{text}"
                                            }
                                            if !sources.is_empty() {
                                                div { class: "mt-2 pt-2 border-t border-gray-100 flex flex-wrap gap-1.5",
                                                    {sources.iter().enumerate().map(|(j, src)| rsx! {
                                                        span {
                                                            key: "{j}",
                                                            class: "inline-flex px-2 py-0.5 rounded-full bg-ios-blue/8 text-[11px] text-ios-blue",
                                                            "{src.title}"
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
