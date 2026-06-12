use crate::db::Database;
use crate::services::i18n::t;
use crate::ui::icons::*;
use crate::ui::state::View;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[derive(Props, Clone, PartialEq)]
pub struct ChatMenuProps {
    pub conversation_id: Signal<Option<String>>,
    pub renaming: Signal<bool>,
    pub rename_input: Signal<String>,
    pub confirm_delete: Signal<bool>,
}

#[component]
pub fn ChatMenu(props: ChatMenuProps) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();

    let ChatMenuProps {
        conversation_id,
        mut renaming,
        mut rename_input,
        mut confirm_delete,
    } = props;

    let has_conversation = conversation_id().is_some();
    let lang = (app.current_lang)();

    rsx! {
        div {
            class: "fixed inset-0 z-40",
            onclick: move |_| {
                app.show_chat_menu.set(false);
                renaming.set(false);
            },
        }
        div {
            class: "absolute right-4 top-1 z-50 bg-warm-white rounded-xl shadow-lg border border-stone-200 p-1 min-w-[220px]",
            style: "animation: popIn 0.16s ease-out; transform-origin: top right;",
            if renaming() {
                div { class: "px-4 py-3",
                    p { class: "text-[10px] font-medium text-stone-400 uppercase tracking-wide mb-2", {t(&lang, "chat-menu-rename")} }
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
                            {t(&lang, "chat-menu-cancel")}
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
                            {t(&lang, "chat-menu-ok")}
                        }
                    }
                }
            } else if confirm_delete() {
                div { class: "px-4 py-3",
                    p { class: "text-[10px] font-medium text-stone-400 uppercase tracking-wide mb-1", {t(&lang, "chat-menu-delete-title")} }
                    p { class: "text-xs text-stone-500 mb-3", {t(&lang, "chat-menu-delete-warning")} }
                    div { class: "flex gap-2",
                        button {
                            class: "flex-1 h-9 text-sm font-medium text-stone-900 bg-stone-100 rounded-full active:bg-stone-200",
                            onclick: move |_| {
                                confirm_delete.set(false);
                                app.show_chat_menu.set(false);
                            },
                            {t(&lang, "chat-menu-cancel")}
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
                            {t(&lang, "chat-menu-delete")}
                        }
                    }
                }
            } else {
                button {
                    class: "w-full flex items-center gap-2.5 px-3 py-2.5 rounded-lg text-sm text-stone-800 text-left active:bg-stone-100 hover:bg-stone-100 transition-colors duration-150",
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
                    {t(&lang, "chat-menu-rename-action")}
                }
                div { class: "h-px bg-stone-100 my-1 mx-2" }
                button {
                    class: "w-full flex items-center gap-2.5 px-3 py-2.5 rounded-lg text-sm text-ios-red text-left active:bg-ios-red/10 hover:bg-ios-red/10 transition-colors duration-150",
                    disabled: !has_conversation,
                    onclick: move |_| {
                        confirm_delete.set(true);
                    },
                    IconTrash { size: 18 }
                    {t(&lang, "chat-menu-delete-chat")}
                }
            }
        }
    }
}
