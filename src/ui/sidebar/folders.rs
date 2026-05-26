use crate::db::Database;
use crate::models::{Folder, NewFolder, UpdateFolder};
use crate::services::i18n::t;
use crate::ui::icons::*;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn FolderSection() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let mut creating = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let lang = (app.current_lang)();

    let folders = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_root_folders().unwrap_or_default()
    });

    rsx! {
        div { class: "flex items-center justify-between px-2 mb-2",
            span { class: "text-xs font-medium text-stone-400 uppercase tracking-wide", {t(&lang, "sidebar-folders-title")} }
            button {
                class: "w-9 h-9 flex items-center justify-center rounded-full text-stone-500",
                onclick: move |_| creating.set(!creating()),
                IconPlus { size: 18 }
            }
        }
        if creating() {
            div { class: "flex items-center gap-2 px-2 mb-2",
                input {
                    class: "flex-1 text-sm border border-stone-200 rounded-lg px-2 py-1.5 outline-none",
                    placeholder: t(&lang, "sidebar-folder-placeholder"),
                    value: "{new_name}",
                    oninput: move |evt| new_name.set(evt.value()),
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter && !new_name().trim().is_empty() {
                            let folder = NewFolder {
                                name: new_name().trim().to_string(),
                                description: None,
                                parent_id: None,
                            };
                            let _ = db().create_folder(&folder);
                            new_name.set(String::new());
                            creating.set(false);
                            app.folders_version.set((app.folders_version)() + 1);
                        }
                    },
                }
                button {
                    class: "w-9 h-9 flex items-center justify-center rounded-lg bg-ios-orange text-white",
                    onclick: move |_| {
                        if !new_name().trim().is_empty() {
                            let folder = NewFolder {
                                name: new_name().trim().to_string(),
                                description: None,
                                parent_id: None,
                            };
                            let _ = db().create_folder(&folder);
                            new_name.set(String::new());
                            creating.set(false);
                            app.folders_version.set((app.folders_version)() + 1);
                        }
                    },
                    IconCheck { size: 18 }
                }
            }
        }
        if folders().is_empty() && !creating() {
            p { class: "text-xs text-stone-400 px-2 py-3", {t(&lang, "sidebar-no-folders")} }
        }
        for folder in folders() {
            FolderItem { folder: folder, depth: 0 }
        }
    }
}

#[component]
fn FolderItem(folder: Folder, depth: u32) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let mut expanded = use_signal(|| false);
    let mut creating_sub = use_signal(|| false);
    let mut sub_name = use_signal(String::new);
    let mut editing = use_signal(|| false);
    let mut edit_name = use_signal(|| folder.name.clone());
    let mut confirm_delete = use_signal(|| false);
    let mut show_actions = use_signal(|| false);
    let lang = (app.current_lang)();

    let folder_id = folder.id.clone();
    let folder_id_for_delete = folder.id.clone();
    let folder_id_for_sub = folder.id.clone();
    let folder_id_for_sub2 = folder.id.clone();
    let folder_id_for_rename = folder.id.clone();
    let folder_id_for_rename2 = folder.id.clone();

    let children = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_subfolders(&folder_id).unwrap_or_default()
    });
    let has_children = !children().is_empty();

    let folder_id_nav = folder.id.clone();
    let is_selected =
        (app.selected_folder_id)().as_deref() == Some(folder_id_nav.as_str());

    let margin = format!("margin-left: {}px", depth * 16);

    rsx! {
        div { style: "{margin}",
            if editing() {
                div { class: "flex items-center gap-2 py-1",
                    input {
                        class: "flex-1 text-sm border border-stone-200 rounded-lg px-2 py-1.5 outline-none",
                        value: "{edit_name}",
                        oninput: move |evt| edit_name.set(evt.value()),
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter && !edit_name().trim().is_empty() {
                                let upd = UpdateFolder {
                                    name: Some(edit_name().trim().to_string()),
                                    description: None,
                                    parent_id: None,
                                };
                                let _ = db().update_folder(&folder_id_for_rename, &upd);
                                editing.set(false);
                                app.folders_version.set((app.folders_version)() + 1);
                            }
                        },
                    }
                    button {
                        class: "w-8 h-8 flex items-center justify-center rounded-lg bg-ios-orange text-white text-xs font-medium",
                        onclick: move |_| {
                            if !edit_name().trim().is_empty() {
                                let upd = UpdateFolder {
                                    name: Some(edit_name().trim().to_string()),
                                    description: None,
                                    parent_id: None,
                                };
                                let _ = db().update_folder(&folder_id_for_rename2, &upd);
                                editing.set(false);
                                app.folders_version.set((app.folders_version)() + 1);
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
                            let _ = db().delete_folder(&folder_id_for_delete);
                            if (app.selected_folder_id)().as_deref() == Some(folder_id_for_delete.as_str()) {
                                app.selected_folder_id.set(None);
                            }
                            app.folders_version.set((app.folders_version)() + 1);
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
                    if has_children {
                        button {
                            class: "min-w-[32px] min-h-[44px] flex items-center justify-center",
                            onclick: move |_| expanded.set(!expanded()),
                            div {
                                class: "w-1.5 h-1.5 border-r-2 border-b-2 border-stone-400 transition-transform duration-150",
                                class: if expanded() { "rotate-45" } else { "-rotate-45" },
                            }
                        }
                    } else {
                        div { class: "w-8 min-w-[32px]" }
                    }
                    button {
                        class: "flex-1 flex items-center gap-2 text-left px-2 py-2.5 text-sm text-stone-900 rounded-lg min-h-[44px]",
                        class: if is_selected { "bg-stone-100" },
                        onclick: move |_| {
                            app.selected_folder_id.set(Some(folder_id_nav.clone()));
                            app.view.set(View::NotesList);
                            app.sidebar_open.set(false);
                        },
                        IconFolder { size: 16 }
                        "{folder.name}"
                    }
                    button {
                        class: "w-9 h-9 flex items-center justify-center text-stone-400",
                        onclick: move |_| show_actions.set(!show_actions()),
                        IconDotsThree { size: 20 }
                    }
                }
                if show_actions() {
                    div { class: "flex items-center gap-0 px-2 py-1 ml-8",
                        button {
                            class: "w-10 h-10 flex items-center justify-center text-stone-500",
                            onclick: move |_| {
                                show_actions.set(false);
                                editing.set(true);
                            },
                            IconPencil { size: 18 }
                        }
                        button {
                            class: "w-10 h-10 flex items-center justify-center text-stone-500",
                            onclick: move |_| {
                                show_actions.set(false);
                                creating_sub.set(true);
                            },
                            IconFolderPlus { size: 18 }
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
            if creating_sub() {
                div { class: "flex items-center gap-2 px-2 py-1 ml-8",
                    input {
                        class: "flex-1 text-sm border border-stone-200 rounded-lg px-2 py-1.5 outline-none",
                        placeholder: t(&lang, "sidebar-subfolder-placeholder"),
                        value: "{sub_name}",
                        oninput: move |evt| sub_name.set(evt.value()),
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter && !sub_name().trim().is_empty() {
                                let folder = NewFolder {
                                    name: sub_name().trim().to_string(),
                                    description: None,
                                    parent_id: Some(folder_id_for_sub.clone()),
                                };
                                let _ = db().create_folder(&folder);
                                sub_name.set(String::new());
                                creating_sub.set(false);
                                expanded.set(true);
                                app.folders_version.set((app.folders_version)() + 1);
                            }
                        },
                    }
                    button {
                        class: "w-8 h-8 flex items-center justify-center rounded-lg bg-ios-orange text-white text-xs font-medium",
                        onclick: move |_| {
                            if !sub_name().trim().is_empty() {
                                let folder = NewFolder {
                                    name: sub_name().trim().to_string(),
                                    description: None,
                                    parent_id: Some(folder_id_for_sub2.clone()),
                                };
                                let _ = db().create_folder(&folder);
                                sub_name.set(String::new());
                                creating_sub.set(false);
                                expanded.set(true);
                                app.folders_version.set((app.folders_version)() + 1);
                            }
                        },
                        IconCheck { size: 18 }
                    }
                }
            }
            if expanded() || has_children {
                if expanded() {
                    for child in children() {
                        FolderItem { folder: child, depth: depth + 1 }
                    }
                }
            }
        }
    }
}
