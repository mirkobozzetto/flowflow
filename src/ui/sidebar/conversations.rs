use crate::db::Database;
use crate::services::i18n::t;
use crate::ui::icons::*;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ConversationSection() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let version = use_signal(|| 0u32);
    let lang = (app.current_lang)();

    let conversations = use_memo(move || {
        let _v = version();
        let _sv = (app.sync_data_version)();
        db().list_conversations().unwrap_or_default()
    });

    rsx! {
        button {
            class: "flex items-center gap-2 w-full px-2 py-3 text-sm font-medium text-ios-orange-dark rounded-lg min-h-[44px] mb-2",
            onclick: move |_| {
                app.view.set(View::Chat { conversation_id: None });
                app.sidebar_open.set(false);
            },
            IconPlus { size: 16 }
            {t(&lang, "sidebar-new-conversation")}
        }
        if conversations().is_empty() {
            p { class: "text-xs text-stone-400 px-2 py-3", {t(&lang, "sidebar-no-conversations")} }
        }
        for conv in conversations() {
            ConversationItem { conv: conv, version: version }
        }
    }
}

#[component]
fn ConversationItem(
    conv: crate::models::conversation::Conversation,
    version: Signal<u32>,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let mut show_actions = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    let mut editing = use_signal(|| false);
    let mut edit_name = use_signal(|| conv.title.clone());
    let lang = (app.current_lang)();

    let conv_id_nav = conv.id.clone();
    let conv_id_rename = conv.id.clone();
    let conv_id_rename2 = conv.id.clone();
    let conv_id_del = conv.id.clone();
    let title = if conv.title.is_empty() {
        t(&lang, "sidebar-untitled")
    } else {
        conv.title.clone()
    };
    let date = if conv.modified_at.len() >= 10 {
        conv.modified_at[..10].to_string()
    } else {
        conv.modified_at.clone()
    };

    rsx! {
        if editing() {
            div { class: "flex items-center gap-2 py-1",
                input {
                    class: "flex-1 text-sm border border-stone-200 rounded-lg px-2 py-1.5 outline-none",
                    value: "{edit_name}",
                    oninput: move |evt| edit_name.set(evt.value()),
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter && !edit_name().trim().is_empty() {
                            let _ = db().update_conversation_title(&conv_id_rename, edit_name().trim());
                            editing.set(false);
                            version.set(version() + 1);
                        }
                    },
                }
                button {
                    class: "w-8 h-8 flex items-center justify-center rounded-lg bg-ios-orange text-white",
                    onclick: move |_| {
                        if !edit_name().trim().is_empty() {
                            let _ = db().update_conversation_title(&conv_id_rename2, edit_name().trim());
                            editing.set(false);
                            version.set(version() + 1);
                        }
                    },
                    IconCheck { size: 18 }
                }
                button {
                    class: "w-9 h-9 flex items-center justify-center text-stone-400",
                    onclick: move |_| editing.set(false),
                    IconX { size: 16 }
                }
            }
        } else if confirm_delete() {
            div { class: "flex items-center gap-2 py-2 px-2",
                span { class: "flex-1 text-sm text-stone-600", {t(&lang, "sidebar-delete-confirm")} }
                button {
                    class: "px-3 py-1 rounded-lg bg-ios-red text-white text-xs font-medium",
                    onclick: move |_| {
                        let _ = db().delete_conversation(&conv_id_del);
                        version.set(version() + 1);
                    },
                    {t(&lang, "sidebar-yes")}
                }
                button {
                    class: "px-3 py-1 rounded-lg bg-stone-200 text-stone-600 text-xs",
                    onclick: move |_| confirm_delete.set(false),
                    {t(&lang, "sidebar-no")}
                }
            }
        } else {
            div { class: "flex items-center",
                button {
                    class: "flex-1 text-left px-2 py-2.5 rounded-lg min-h-[44px]",
                    onclick: move |_| {
                        app.view.set(View::Chat { conversation_id: Some(conv_id_nav.clone()) });
                        app.sidebar_open.set(false);
                    },
                    p { class: "text-sm text-stone-900 line-clamp-1", "{title}" }
                    p { class: "text-[10px] text-stone-400 mt-0.5", "{date}" }
                }
                button {
                    class: "w-9 h-9 flex items-center justify-center text-stone-400",
                    onclick: move |_| show_actions.set(!show_actions()),
                    IconDotsThree { size: 20 }
                }
            }
            if show_actions() {
                div { class: "flex items-center gap-0 px-2 py-1 ml-2",
                    button {
                        class: "w-10 h-10 flex items-center justify-center text-stone-500",
                        onclick: move |_| {
                            show_actions.set(false);
                            editing.set(true);
                        },
                        IconPencil { size: 18 }
                    }
                    button {
                        class: "w-10 h-10 flex items-center justify-center text-stone-400",
                        onclick: move |_| {
                            show_actions.set(false);
                            confirm_delete.set(true);
                        },
                        IconTrash { size: 18 }
                    }
                }
            }
        }
    }
}
