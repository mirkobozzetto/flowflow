use crate::application::i18n::t;
use crate::domain::UpdateThread;
use crate::infrastructure::persistence::Database;
use crate::ui::delete_confirm::DeleteConfirm;
use crate::ui::icons::*;
use crate::ui::kit;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ThreadHeaderMenu(thread_id: String) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let mut renaming = use_signal(|| false);
    // The top-bar theme tap opens this menu straight on the theme chooser.
    let mut choosing_theme = use_signal(|| *app.show_thread_theme.peek());
    let mut confirm_delete = use_signal(|| false);
    use_drop(move || {
        let mut app = app;
        app.show_thread_theme.set(false);
    });
    let current_title = db()
        .get_thread(&thread_id)
        .ok()
        .flatten()
        .map(|t| t.title)
        .unwrap_or_default();
    let mut rename_input = use_signal(|| current_title.clone());

    let folders = {
        let _ = (app.folders_version)();
        db().list_all_folders().unwrap_or_default()
    };

    rsx! {
        div {
            class: "fixed inset-0 z-40",
            onclick: move |_| app.show_thread_menu.set(false),
        }
        div { class: "absolute right-4 top-1 {kit::MENU_PANEL}",
            if renaming() {
                div { class: "px-2 py-1.5",
                    p { class: "{kit::SECTION_LABEL} mb-2 px-1", {t(&lang, "thread-menu-rename")} }
                    div { class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1",
                        input {
                            class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900",
                            value: "{rename_input}",
                            oninput: move |evt| rename_input.set(evt.value()),
                            onkeypress: {
                                let tid = thread_id.clone();
                                move |evt: Event<KeyboardData>| {
                                    if evt.key() == Key::Enter {
                                        let v = rename_input().trim().to_string();
                                        if !v.is_empty() {
                                            let _ = db().update_thread(&tid, &UpdateThread { title: Some(v), folder_id: None });
                                            app.notes_version.set((app.notes_version)() + 1);
                                        }
                                        app.show_thread_menu.set(false);
                                    }
                                }
                            },
                        }
                        button {
                            class: "w-10 h-10 shrink-0 flex items-center justify-center rounded-lg transition-colors duration-150",
                            class: if rename_input().trim().is_empty() { "text-stone-300" } else { "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80" },
                            onclick: {
                                let tid = thread_id.clone();
                                move |_| {
                                    let v = rename_input().trim().to_string();
                                    if !v.is_empty() {
                                        let _ = db().update_thread(&tid, &UpdateThread { title: Some(v), folder_id: None });
                                        app.notes_version.set((app.notes_version)() + 1);
                                    }
                                    app.show_thread_menu.set(false);
                                }
                            },
                            IconCheck { size: 16 }
                        }
                    }
                }
            } else if choosing_theme() {
                div { class: "px-1 py-1 max-h-[60vh] overflow-y-auto",
                    p { class: "{kit::SECTION_LABEL} mb-1 px-3 pt-1.5", {t(&lang, "thread-menu-theme")} }
                    button {
                        class: kit::MENU_ITEM,
                        onclick: {
                            let tid = thread_id.clone();
                            move |_| {
                                let _ = db().update_thread(&tid, &UpdateThread { title: None, folder_id: Some(None) });
                                app.notes_version.set((app.notes_version)() + 1);
                                app.show_thread_menu.set(false);
                            }
                        },
                        {t(&lang, "thread-theme-none")}
                    }
                    for folder in folders.iter() {
                        button {
                            key: "{folder.id}",
                            class: kit::MENU_ITEM,
                            onclick: {
                                let tid = thread_id.clone();
                                let fid = folder.id.clone();
                                move |_| {
                                    let _ = db().update_thread(&tid, &UpdateThread { title: None, folder_id: Some(Some(fid.clone())) });
                                    app.notes_version.set((app.notes_version)() + 1);
                                    app.show_thread_menu.set(false);
                                }
                            },
                            IconFolder { size: 16 }
                            "{folder.name}"
                        }
                    }
                }
            } else if confirm_delete() {
                DeleteConfirm {
                    title: t(&lang, "thread-menu-delete-title"),
                    warning: t(&lang, "thread-menu-delete-warning"),
                    cancel_label: t(&lang, "thread-menu-cancel"),
                    confirm_label: t(&lang, "thread-menu-delete"),
                    on_cancel: move |_| app.show_thread_menu.set(false),
                    on_confirm: {
                        let tid = thread_id.clone();
                        move |_| {
                            let _ = db().delete_thread(&tid);
                            app.notes_version.set((app.notes_version)() + 1);
                            app.show_thread_menu.set(false);
                            app.view.set(View::NotesList);
                        }
                    },
                }
            } else {
                button {
                    class: kit::MENU_ITEM,
                    onclick: move |_| renaming.set(true),
                    IconPencil { size: 16 }
                    {t(&lang, "thread-menu-rename")}
                }
                button {
                    class: kit::MENU_ITEM,
                    onclick: move |_| choosing_theme.set(true),
                    IconFolder { size: 16 }
                    {t(&lang, "thread-menu-theme")}
                }
                div { class: kit::MENU_SEP }
                button {
                    class: kit::MENU_ITEM_DANGER,
                    onclick: move |_| confirm_delete.set(true),
                    IconTrash { size: 16 }
                    {t(&lang, "thread-menu-delete")}
                }
            }
        }
    }
}
