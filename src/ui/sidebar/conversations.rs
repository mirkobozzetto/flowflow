use crate::db::Database;
use crate::services::i18n::t;
use crate::ui::icons::*;
use crate::ui::kit;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ConversationSection() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let version = use_signal(|| 0u32);
    let mut chat_query = use_signal(String::new);
    let lang = (app.current_lang)();

    let conversations = use_memo(move || {
        let _v = version();
        let _sv = (app.sync_data_version)();
        let all = db().list_conversations().unwrap_or_default();
        let q = chat_query().to_lowercase();
        if q.is_empty() {
            return all;
        }
        all.into_iter()
            .filter(|c| c.title.to_lowercase().contains(&q))
            .collect()
    });

    let has_query = !chat_query().is_empty();

    rsx! {
        button {
            class: "flex items-center gap-2 w-full px-2 py-3 text-sm font-medium text-ios-orange-dark rounded-lg min-h-[44px] mb-2 hover:bg-ios-orange-50 transition-colors duration-150",
            onclick: move |_| {
                app.sidebar_open.set(false);
                crate::ui::sidebar::navigate_with_slide(
                    app,
                    View::Chat { conversation_id: None },
                );
            },
            IconPlus { size: 16 }
            {t(&lang, "sidebar-new-conversation")}
        }
        div { class: "relative mb-2",
            div { class: "absolute left-3 top-1/2 -translate-y-1/2 text-stone-400",
                IconMagnifyingGlass { size: 14 }
            }
            input {
                class: "w-full bg-stone-100 rounded-lg pl-8 pr-8 py-2 text-sm outline-none text-stone-900 placeholder-stone-400 focus:ring-[3px] focus:ring-ios-orange-50 transition-colors duration-150",
                placeholder: t(&lang, "sidebar-search-chats"),
                value: "{chat_query}",
                oninput: move |evt| chat_query.set(evt.value()),
            }
            if has_query {
                button {
                    class: "absolute right-2 top-1/2 -translate-y-1/2 text-stone-400 active:text-stone-600 hover:text-stone-600 p-1 transition-colors duration-150",
                    onclick: move |_| chat_query.set(String::new()),
                    IconX { size: 12 }
                }
            }
        }
        if conversations().is_empty() {
            p { class: "text-xs text-stone-400 px-2 py-3",
                if has_query {
                    {t(&lang, "note-list-no-results")}
                } else {
                    {t(&lang, "sidebar-no-conversations")}
                }
            }
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
            div {
                class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1 my-1",
                style: "animation: popIn 0.16s ease-out;",
                input {
                    class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900",
                    value: "{edit_name}",
                    oninput: move |evt| edit_name.set(evt.value()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Escape {
                            editing.set(false);
                        }
                    },
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter && !edit_name().trim().is_empty() {
                            let _ = db().update_conversation_title(&conv_id_rename, edit_name().trim());
                            editing.set(false);
                            version.set(version() + 1);
                        }
                    },
                }
                button {
                    class: "w-10 h-10 shrink-0 flex items-center justify-center rounded-lg transition-colors duration-150",
                    class: if edit_name().trim().is_empty() {
                        "text-stone-300"
                    } else {
                        "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                    },
                    onclick: move |_| {
                        if !edit_name().trim().is_empty() {
                            let _ = db().update_conversation_title(&conv_id_rename2, edit_name().trim());
                            editing.set(false);
                            version.set(version() + 1);
                        }
                    },
                    IconCheck { size: 16 }
                }
                button {
                    class: "w-10 h-10 shrink-0 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors duration-150",
                    onclick: move |_| editing.set(false),
                    IconX { size: 14 }
                }
            }
        } else {
            div { class: "flex items-center group relative",
                button {
                    class: "flex-1 text-left px-2 py-2.5 rounded-lg min-h-[44px] hover:bg-stone-100 transition-colors duration-150",
                    onclick: move |_| {
                        app.sidebar_open.set(false);
                        crate::ui::sidebar::navigate_with_slide(
                            app,
                            View::Chat { conversation_id: Some(conv_id_nav.clone()) },
                        );
                    },
                    p { class: "text-sm text-stone-900 line-clamp-1", "{title}" }
                    p { class: "text-xs text-stone-500 mt-0.5", "{date}" }
                }
                button {
                    class: "w-11 h-11 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-all duration-150",
                    class: if show_actions() {
                        "text-stone-600"
                    } else {
                        "lg:opacity-0 lg:group-hover:opacity-100"
                    },
                    onclick: move |_| show_actions.set(!show_actions()),
                    IconDotsThree { size: 20 }
                }
                if show_actions() {
                    div {
                        class: "fixed inset-0 z-40",
                        onclick: move |_| {
                            show_actions.set(false);
                            confirm_delete.set(false);
                        },
                    }
                    div { class: "absolute right-2 top-full {kit::MENU_PANEL}",
                        if confirm_delete() {
                            div { class: "px-3 py-2.5",
                                p { class: "text-sm font-semibold text-stone-900 mb-0.5",
                                    {t(&lang, "chat-menu-delete-title")}
                                }
                                p { class: "text-xs text-stone-500 mb-3",
                                    {t(&lang, "chat-menu-delete-warning")}
                                }
                                div { class: "flex gap-2",
                                    button {
                                        class: kit::CONFIRM_BTN_GHOST,
                                        onclick: move |_| {
                                            confirm_delete.set(false);
                                            show_actions.set(false);
                                        },
                                        {t(&lang, "chat-menu-cancel")}
                                    }
                                    button {
                                        class: kit::CONFIRM_BTN_DANGER,
                                        onclick: move |_| {
                                            let _ = db().delete_conversation(&conv_id_del);
                                            version.set(version() + 1);
                                            confirm_delete.set(false);
                                            show_actions.set(false);
                                        },
                                        {t(&lang, "chat-menu-delete")}
                                    }
                                }
                            }
                        } else {
                            button {
                                class: kit::MENU_ITEM,
                                onclick: move |_| {
                                    show_actions.set(false);
                                    editing.set(true);
                                },
                                IconPencil { size: 16 }
                                {t(&lang, "folder-menu-rename")}
                            }
                            div { class: kit::MENU_SEP }
                            button {
                                class: kit::MENU_ITEM_DANGER,
                                onclick: move |_| confirm_delete.set(true),
                                IconTrash { size: 16 }
                                {t(&lang, "folder-menu-delete")}
                            }
                        }
                    }
                }
            }
        }
    }
}
