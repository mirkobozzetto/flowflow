use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::tools_menu::{LEAD_ICON, ROW, ROW_TITLE, SECTION, SEP};
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub struct MentionedNote {
    pub note_id: String,
    pub title: String,
}

// Mentions are detected at the end of the input ("... @frag"). Selecting an item
// replaces that trailing "@frag" with the chosen token (or removes it for a tool).
fn replace_trailing_at(s: &str, replacement: &str) -> String {
    match s.rfind('@') {
        Some(at) => format!("{}{}", &s[..at], replacement),
        None => format!("{s}{replacement}"),
    }
}

#[component]
pub fn MentionMenu(
    input: Signal<String>,
    mentions: Signal<Vec<MentionedNote>>,
    query: String,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let q = query.to_lowercase();
    let notes: Vec<(String, String)> = db()
        .list_notes()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|n| {
            let title = n.title.unwrap_or_default();
            if title.is_empty() {
                None
            } else {
                Some((n.id, title))
            }
        })
        .filter(|(_, title)| q.is_empty() || title.to_lowercase().contains(&q))
        .take(8)
        .collect();

    rsx! {
        div { class: "absolute bottom-full left-2 right-2 lg:left-0 lg:right-auto lg:w-72 mb-2 z-40 max-h-64 overflow-y-auto bg-warm-white border border-stone-200 rounded-xl shadow-lg p-2 popover-pop",
            div { class: SECTION, {t(&lang, "chat-tools-section-tools")} }
            button {
                class: ROW,
                onclick: move |_| {
                    app.chat_web.set(true);
                    input.set(replace_trailing_at(&input(), ""));
                    app.show_mention_menu.set(false);
                },
                span { class: LEAD_ICON, IconGlobeSimple { size: 22 } }
                span { class: ROW_TITLE, {t(&lang, "chat-tools-web")} }
            }
            // Parked until implemented (icon + i18n kept): deep search.
            /*
            div { class: "w-full flex items-center gap-3 px-3 min-h-[48px] rounded-lg opacity-50",
                span { class: LEAD_ICON, IconMagnifyingGlass { size: 22 } }
                span { class: ROW_TITLE, {t(&lang, "chat-tools-deep")} }
            }
            */
            div { class: SEP }
            div { class: SECTION, {t(&lang, "chat-mention-section-notes")} }
            if notes.is_empty() {
                div { class: "px-3 py-2 text-sm text-stone-400",
                    {t(&lang, "chat-mention-no-notes")}
                }
            } else {
                for (id, title) in notes {
                    button {
                        key: "{id}",
                        class: ROW,
                        onclick: {
                            let id = id.clone();
                            let title = title.clone();
                            move |_| {
                                let token = format!("@{title} ");
                                input.set(replace_trailing_at(&input(), &token));
                                let mut next = mentions();
                                if !next.iter().any(|m| m.note_id == id) {
                                    next.push(MentionedNote {
                                        note_id: id.clone(),
                                        title: title.clone(),
                                    });
                                }
                                mentions.set(next);
                                app.show_mention_menu.set(false);
                            }
                        },
                        span { class: LEAD_ICON, IconNotebook { size: 22 } }
                        span { class: "flex-1 min-w-0 truncate {ROW_TITLE}", "{title}" }
                    }
                }
            }
        }
    }
}
