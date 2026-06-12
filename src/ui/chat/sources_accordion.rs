use crate::services::i18n::{t, t_args};
use crate::ui::chat::models::ChatSource;
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn SourcesAccordion(
    sources: Vec<ChatSource>,
    conversation_id: Option<String>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let label = if sources.len() == 1 {
        t(&lang, "sources-one")
    } else {
        let count = sources.len().to_string();
        t_args(&lang, "sources-many", &[("count", &count)])
    };
    let mut expanded = use_signal(|| false);
    let rotation = if expanded() { "180" } else { "0" };
    let arrow_style = format!(
        "transition: transform 0.3s ease; transform: rotate({rotation}deg);"
    );

    rsx! {
        div { class: "mt-2 pt-2 border-t border-stone-100",
            button {
                class: "w-full flex items-center justify-between py-1",
                onclick: move |_| {
                    let opening = !expanded();
                    expanded.set(opening);
                    let offset = if opening { 150 } else { -150 };
                    dioxus::document::eval(&format!(
                        r#"setTimeout(() => {{
                            let el = document.getElementById('chat-messages');
                            if (el) el.scrollBy({{ top: {offset}, behavior: 'smooth' }});
                        }}, 350);"#
                    ));
                },
                span { class: "text-xs font-medium text-stone-500",
                    "{label}"
                }
                span {
                    class: "text-xs text-stone-400 inline-block",
                    style: "{arrow_style}",
                    "▼"
                }
            }
            div {
                class: if expanded() {
                    "flex flex-col gap-1.5 mt-1 overflow-hidden max-h-96 opacity-100 transition-all duration-300 ease-in-out"
                } else {
                    "flex flex-col gap-1.5 mt-1 overflow-hidden max-h-0 opacity-0 transition-all duration-300 ease-in-out"
                },
                {sources.iter().enumerate().map(|(j, src)| {
                    rsx! {
                        SourceCard {
                            key: "{j}",
                            source: src.clone(),
                            conversation_id: conversation_id.clone(),
                        }
                    }
                })}
            }
        }
    }
}

#[component]
fn SourceCard(source: ChatSource, conversation_id: Option<String>) -> Element {
    let mut app: AppState = use_context();
    let nid = source.note_id.clone();
    let preview = if source.chunk_text.chars().count() > 80 {
        let s: String = source.chunk_text.chars().take(80).collect();
        format!("{s}...")
    } else {
        source.chunk_text.clone()
    };
    let date_label = if source.created_at.len() >= 16 {
        let d = &source.created_at[..10];
        let h = &source.created_at[11..16];
        format!("{d} · {h}")
    } else if !source.created_at.is_empty() {
        source.created_at[..10.min(source.created_at.len())].to_string()
    } else {
        String::new()
    };

    rsx! {
        button {
            class: "w-full text-left px-2.5 py-1.5 rounded-lg bg-warm-white border border-ios-orange/20 active:bg-ios-orange/10 hover:bg-ios-orange/5 transition-colors duration-150",
            onclick: move |_| {
                app.previous_view.set(Some(View::Chat {
                    conversation_id: conversation_id.clone(),
                }));
                app.view.set(View::NoteDetail {
                    note_id: nid.clone(),
                });
            },
            div { class: "flex items-center justify-between",
                span { class: "text-xs font-medium text-ios-orange-dark",
                    "{source.title}"
                }
                if !date_label.is_empty() {
                    span { class: "text-xs text-stone-500 ml-1 shrink-0",
                        "{date_label}"
                    }
                }
            }
            p { class: "text-xs text-stone-500 leading-tight mt-0.5 line-clamp-1",
                "{preview}"
            }
        }
    }
}
