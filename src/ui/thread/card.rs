use crate::application::i18n::t;
use crate::domain::Thread;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{IconArrowUpRight, IconBell};
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ThreadCard(thread: Thread) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let _ = (app.notes_version)();
    let tid = thread.id.clone();

    let members = db().list_thread_notes(&thread.id).unwrap_or_default();
    let preview = members
        .last()
        .map(|n| {
            if n.content.chars().count() > 120 {
                format!(
                    "{}...",
                    crate::application::ai::char_prefix(&n.content, 120)
                )
            } else {
                n.content.clone()
            }
        })
        .unwrap_or_default();

    let has_web = members.iter().any(|n| n.sources_json.is_some());
    let has_audio = members.iter().any(|n| db().has_any_audio(&n.id));
    let has_reminder = members.iter().any(|n| {
        db().reminders_for_note(&n.id)
            .into_iter()
            .any(|r| r.state == "active")
    });

    let date = thread.modified_at.get(..10).unwrap_or("").to_string();
    let theme = thread
        .folder_id
        .as_ref()
        .and_then(|fid| db().get_folder(fid).ok().flatten().map(|f| f.name));
    let title = if thread.title.is_empty() {
        t(&lang, "thread-untitled")
    } else {
        thread.title.clone()
    };

    rsx! {
        div { class: "relative mb-3 break-inside-avoid lg:mb-0 lg:h-full lg:flex lg:flex-col",
            div { class: "absolute inset-x-0 top-3 h-full bg-warm-white border border-stone-200 rounded-xl scale-x-[0.90] opacity-30 shadow-card" }
            div { class: "absolute inset-x-0 top-1.5 h-full bg-warm-white border border-stone-200 rounded-xl scale-x-[0.95] opacity-60 shadow-card" }
            div {
                class: "relative lg:flex-1 bg-warm-white p-4 border border-stone-200 rounded-xl cursor-pointer break-inside-avoid shadow-card hover:border-stone-300 hover:shadow-lift hover:-translate-y-px transition-[box-shadow,transform,border-color] duration-180",
                onclick: move |_| {
                    app.show_folder_picker.set(false);
                    app.previous_view.set(Some(View::NotesList));
                    app.view.set(View::ThreadDetail { thread_id: tid.clone() });
                },
                div { class: "flex justify-between items-center mb-2",
                    h3 { class: "font-semibold text-base text-stone-900", "{title}" }
                    div { class: "flex items-center gap-1.5 shrink-0",
                        if has_web {
                            span { class: "text-ios-orange-dark", IconArrowUpRight { size: 14 } }
                        }
                        if has_reminder {
                            span { class: "text-ios-orange-dark", IconBell { size: 14 } }
                        }
                        if has_audio {
                            div { class: "w-2 h-2 rounded-full bg-ios-orange/50" }
                        }
                    }
                }
                if !preview.is_empty() {
                    p { class: "text-stone-600 text-sm mb-2 line-clamp-2", "{preview}" }
                }
                div { class: "flex items-center gap-2",
                    span { class: "text-stone-500 text-xs", "{date}" }
                    if let Some(ref tname) = theme {
                        span { class: "text-xs text-stone-400", "·" }
                        span { class: "text-xs text-ios-orange-dark", "{tname}" }
                    }
                }
            }
        }
    }
}
