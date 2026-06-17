use crate::db::Database;
use crate::services::i18n::{t, t_args};
use crate::ui::chat::models::ChatSource;
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn SourcesAccordion(
    sources: Vec<ChatSource>,
    conversation_id: Option<String>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let web: Vec<ChatSource> = sources
        .iter()
        .filter(|s| s.url.is_some())
        .cloned()
        .collect();
    let notes: Vec<ChatSource> = sources
        .iter()
        .filter(|s| s.url.is_none())
        .cloned()
        .collect();

    let web_label = t(&lang, "sources-web");
    let notes_label = if notes.len() == 1 {
        t(&lang, "sources-one")
    } else {
        let count = notes.len().to_string();
        t_args(&lang, "sources-many", &[("count", &count)])
    };

    let mut web_open = use_signal(|| true);
    let mut notes_open = use_signal(|| false);
    let web_arrow = format!(
        "transition: transform 0.3s ease; transform: rotate({}deg);",
        if web_open() { "180" } else { "0" }
    );
    let notes_arrow = format!(
        "transition: transform 0.3s ease; transform: rotate({}deg);",
        if notes_open() { "180" } else { "0" }
    );

    rsx! {
        div { class: "mt-2 pt-2 border-t border-stone-100 space-y-2",
            if !web.is_empty() {
                div {
                    button {
                        class: "w-full flex items-center justify-between py-1.5",
                        onclick: move |_| {
                            let v = !web_open();
                            web_open.set(v);
                        },
                        span { class: "text-xs font-medium text-stone-500",
                            "{web_label}"
                        }
                        span {
                            class: "text-xs text-stone-400 inline-block",
                            style: "{web_arrow}",
                            "▼"
                        }
                    }
                    if web_open() {
                        div { class: "flex flex-col gap-2 mt-1",
                            for (j, src) in web.iter().enumerate() {
                                WebSourceCard { key: "w{j}", source: src.clone() }
                            }
                        }
                    }
                }
            }
            if !notes.is_empty() {
                div {
                    button {
                        class: "w-full flex items-center justify-between py-1.5",
                        onclick: move |_| {
                            let opening = !notes_open();
                            notes_open.set(opening);
                            let offset = if opening { 150 } else { -150 };
                            dioxus::document::eval(&format!(
                                r#"setTimeout(() => {{
                                    let el = document.getElementById('chat-messages');
                                    if (el) el.scrollBy({{ top: {offset}, behavior: 'smooth' }});
                                }}, 350);"#
                            ));
                        },
                        span { class: "text-xs font-medium text-stone-500",
                            "{notes_label}"
                        }
                        span {
                            class: "text-xs text-stone-400 inline-block",
                            style: "{notes_arrow}",
                            "▼"
                        }
                    }
                    if notes_open() {
                        div { class: "flex flex-col gap-1.5 mt-1",
                            for (j, src) in notes.iter().enumerate() {
                                SourceCard {
                                    key: "n{j}",
                                    source: src.clone(),
                                    conversation_id: conversation_id.clone(),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SourceCard(source: ChatSource, conversation_id: Option<String>) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
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
                let folder = db()
                    .folders_for_note(&nid)
                    .ok()
                    .and_then(|f| f.first().map(|f| f.id.clone()));
                app.detail_folder_id.set(folder);
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

#[component]
fn WebSourceCard(source: ChatSource) -> Element {
    let mut expanded = use_signal(|| false);
    let url = source.url.clone().unwrap_or_default();
    let domain = url
        .split("://")
        .nth(1)
        .unwrap_or(&url)
        .split('/')
        .next()
        .unwrap_or(&url)
        .to_string();
    let url_for_open = url.clone();

    rsx! {
        div {
            class: if expanded() {
                "w-full rounded-lg bg-warm-white border border-ios-orange/30 overflow-hidden flex flex-col shadow-sm"
            } else {
                "w-full rounded-lg bg-warm-white border border-ios-orange/20 overflow-hidden flex flex-col"
            },
            button {
                class: "w-full text-left px-3 py-2.5 min-h-[44px] active:bg-ios-orange/10 hover:bg-ios-orange/5 transition-colors duration-150 flex items-start gap-2",
                onclick: move |_| {
                    let v = !expanded();
                    expanded.set(v);
                },
                div { class: "flex-1 min-w-0",
                    h4 { class: "text-sm font-medium text-ios-orange-dark line-clamp-2",
                        "{source.title}"
                    }
                    p {
                        class: if expanded() {
                            "text-xs text-stone-600 leading-relaxed whitespace-pre-wrap break-words mt-1.5"
                        } else {
                            "text-xs text-stone-500 leading-tight line-clamp-2 mt-1"
                        },
                        "{source.chunk_text}"
                    }
                }
                svg {
                    width: "16",
                    height: "16",
                    view_box: "0 0 24 24",
                    fill: "none",
                    class: if expanded() {
                        "text-stone-400 shrink-0 mt-0.5 transition-transform duration-300 rotate-180"
                    } else {
                        "text-stone-400 shrink-0 mt-0.5 transition-transform duration-300"
                    },
                    path {
                        d: "M19 9l-7 7-7-7",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                    }
                }
            }
            div { class: "h-px w-full bg-stone-200" }
            button {
                class: "w-full text-left px-3 py-2.5 min-h-[44px] bg-ios-orange/5 active:bg-ios-orange/10 hover:bg-ios-orange/10 transition-colors duration-150 flex items-center justify-between",
                onclick: move |_| {
                    crate::platform::open_url(&url_for_open);
                },
                span { class: "text-xs font-medium text-ios-orange-dark truncate",
                    "{domain}"
                }
                svg {
                    width: "14",
                    height: "14",
                    view_box: "0 0 24 24",
                    fill: "none",
                    class: "text-ios-orange-dark shrink-0 ml-2",
                    path {
                        d: "M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                    }
                }
            }
        }
    }
}
