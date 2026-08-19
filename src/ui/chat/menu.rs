use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::models::ChatSource;
use crate::ui::delete_confirm::DeleteConfirm;
use crate::ui::icons::*;
use crate::ui::kit;
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
        div { class: "absolute right-4 top-1 {kit::MENU_PANEL}",
            if renaming() {
                div { class: "px-2 py-1.5",
                    p { class: "{kit::SECTION_LABEL} mb-2 px-1", {t(&lang, "chat-menu-rename")} }
                    div { class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1",
                        input {
                            class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900",
                            value: "{rename_input}",
                            oninput: move |evt| rename_input.set(evt.value()),
                            onkeydown: move |evt| {
                                if evt.key() == Key::Escape {
                                    renaming.set(false);
                                    app.show_chat_menu.set(false);
                                }
                            },
                            onkeypress: move |evt| {
                                if evt.key() == Key::Enter {
                                    let v = rename_input().trim().to_string();
                                    if !v.is_empty() {
                                        if let Some(ref cid) = conversation_id() {
                                            let _ = db().update_conversation_title(cid, &v);
                                            app.invalidate_data();
                                        }
                                    }
                                    renaming.set(false);
                                    app.show_chat_menu.set(false);
                                }
                            },
                        }
                        button {
                            class: "w-10 h-10 shrink-0 flex items-center justify-center rounded-lg transition-colors duration-150",
                            class: if rename_input().trim().is_empty() {
                                "text-stone-300"
                            } else {
                                "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                            },
                            onclick: move |_| {
                                let v = rename_input().trim().to_string();
                                if !v.is_empty() {
                                    if let Some(ref cid) = conversation_id() {
                                        let _ = db().update_conversation_title(cid, &v);
                                        app.invalidate_data();
                                    }
                                }
                                renaming.set(false);
                                app.show_chat_menu.set(false);
                            },
                            IconCheck { size: 16 }
                        }
                        button {
                            class: "w-10 h-10 shrink-0 flex items-center justify-center text-stone-400 hover:text-stone-600 transition-colors duration-150",
                            onclick: move |_| {
                                renaming.set(false);
                                app.show_chat_menu.set(false);
                            },
                            IconX { size: 14 }
                        }
                    }
                }
            } else if confirm_delete() {
                DeleteConfirm {
                    title: t(&lang, "chat-menu-delete-title"),
                    warning: t(&lang, "chat-menu-delete-warning"),
                    cancel_label: t(&lang, "chat-menu-cancel"),
                    confirm_label: t(&lang, "chat-menu-delete"),
                    on_cancel: move |_| {
                        confirm_delete.set(false);
                        app.show_chat_menu.set(false);
                    },
                    on_confirm: move |_| {
                        if let Some(ref cid) = conversation_id() {
                            let _ = db().delete_conversation(cid);
                        }
                        // The drawer's conversation list stays mounted on mobile (the swipe
                        // panel never unmounts) and only re-reads on this version: without
                        // the bump the deleted chat ghosts in the list.
                        app.invalidate_data();
                        confirm_delete.set(false);
                        app.show_chat_menu.set(false);
                        app.sliding_out.set(true);
                        // spawn_forever, NOT spawn: show_chat_menu=false unmounts THIS menu in
                        // the same click, which cancels a scope-bound task mid-delay - leaving
                        // sliding_out stuck true (whole screen pointer-events-none) and the
                        // view never set. The freeze-after-delete bug.
                        dioxus::core::spawn_forever(async move {
                            futures_timer::Delay::new(std::time::Duration::from_millis(150)).await;
                            app.sliding_out.set(false);
                            app.view.set(View::NotesList);
                        });
                    },
                }
            } else {
                button {
                    class: kit::MENU_ITEM,
                    disabled: !has_conversation,
                    onclick: move |_| {
                        if let Some(ref cid) = conversation_id() {
                            let database = db();
                            if let Ok(msgs) = database.list_messages(cid) {
                                if !msgs.is_empty() {
                                    let user_label =
                                        t(&lang, "chat-transcript-user");
                                    let bot_label =
                                        t(&lang, "chat-transcript-assistant");
                                    let transcript = msgs
                                        .iter()
                                        .map(|m| {
                                            let who = if m.role == "user" {
                                                &user_label
                                            } else {
                                                &bot_label
                                            };
                                            format!("{who}:\n{}", m.content)
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n\n");
                                    let mut web_sources: Vec<ChatSource> =
                                        Vec::new();
                                    let mut seen = std::collections::HashSet::new();
                                    for m in &msgs {
                                        if let Some(ref sj) = m.sources_json {
                                            for s in crate::ui::chat::actions::parse_sources(sj) {
                                                if let Some(ref u) = s.url {
                                                    if seen.insert(u.clone()) {
                                                        web_sources.push(s);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    let folder = crate::ui::chat::actions::scope_save_folder(
                                        &database,
                                        &(app.chat_scope)(),
                                    );
                                    if let Ok(id) = crate::ui::chat::actions::save_as_note(
                                        &database,
                                        folder.as_deref(),
                                        transcript,
                                        vec!["chat".to_string()],
                                        &web_sources,
                                        &lang,
                                    ) {
                                        app.detail_folder_id.set(folder.clone());
                                        app.previous_view.set(Some(View::Chat {
                                            conversation_id: conversation_id(),
                                        }));
                                        app.view.set(View::NoteDetail { note_id: id });
                                    }
                                }
                            }
                        }
                        app.show_chat_menu.set(false);
                    },
                    IconFloppyDisk { size: 16 }
                    {t(&lang, "chat-menu-save-thread")}
                }
                div { class: kit::MENU_SEP }
                button {
                    class: kit::MENU_ITEM,
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
                    IconPencil { size: 16 }
                    {t(&lang, "chat-menu-rename-action")}
                }
                div { class: kit::MENU_SEP }
                button {
                    class: kit::MENU_ITEM_DANGER,
                    disabled: !has_conversation,
                    onclick: move |_| {
                        confirm_delete.set(true);
                    },
                    IconTrash { size: 16 }
                    {t(&lang, "chat-menu-delete-chat")}
                }
            }
        }
    }
}
