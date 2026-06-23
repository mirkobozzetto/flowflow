use crate::application::i18n::t;
use crate::domain::{Folder, NewFolder, UpdateFolder};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::*;
use crate::ui::kit;
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
            span { class: kit::SECTION_LABEL, {t(&lang, "sidebar-folders-title")} }
            button {
                class: "w-11 h-11 flex items-center justify-center rounded-full transition-all duration-200",
                class: if creating() {
                    "rotate-45 text-ios-orange-dark bg-ios-orange-50"
                } else {
                    "text-stone-500 hover:bg-stone-100"
                },
                onclick: move |_| {
                    creating.set(!creating());
                    if creating() {
                        dioxus::document::eval(
                            r#"
                            requestAnimationFrame(function() {
                                var el = document.getElementById('new-theme-input');
                                if (el) el.focus();
                            });
                            "#,
                        );
                    }
                },
                IconPlus { size: 18 }
            }
        }
        div {
            class: "overflow-hidden transition-all duration-200 px-1",
            class: if creating() { "max-h-16 opacity-100 mb-2" } else { "max-h-0 opacity-0" },
            div { class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1",
                input {
                    id: "new-theme-input",
                    class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900 placeholder-stone-400",
                    placeholder: t(&lang, "sidebar-folder-placeholder"),
                    value: "{new_name}",
                    oninput: move |evt| new_name.set(evt.value()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Escape {
                            creating.set(false);
                        }
                    },
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
                    class: "w-10 h-10 flex items-center justify-center rounded-lg transition-colors duration-150",
                    class: if new_name().trim().is_empty() {
                        "text-stone-300"
                    } else {
                        "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                    },
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
                    IconCheck { size: 16 }
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
                        class: "w-10 h-10 shrink-0 flex items-center justify-center rounded-lg transition-colors duration-150",
                        class: if edit_name().trim().is_empty() {
                            "text-stone-300"
                        } else {
                            "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                        },
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
                    if has_children {
                        button {
                            class: "min-w-[32px] min-h-[44px] flex items-center justify-center hover:opacity-70 transition-opacity duration-150",
                            onclick: move |_| expanded.set(!expanded()),
                            div {
                                class: "w-1.5 h-1.5 border-r-2 border-b-2 border-stone-400 chevron-pivot",
                                class: if expanded() { "rotate-45" } else { "-rotate-45" },
                            }
                        }
                    } else {
                        div { class: "w-8 min-w-[32px]" }
                    }
                    button {
                        class: "flex-1 flex items-center gap-2 text-left px-2 py-2.5 text-sm text-stone-900 rounded-lg min-h-[44px] hover:bg-stone-100 transition-colors duration-150",
                        class: if is_selected { "bg-stone-100" },
                        onclick: move |_| {
                            app.selected_folder_id.set(Some(folder_id_nav.clone()));
                            app.sidebar_open.set(false);
                            crate::ui::sidebar::navigate_with_slide(app, View::NotesList);
                        },
                        IconFolder { size: 16 }
                        "{folder.name}"
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
                                        {t(&lang, "folder-menu-delete-title")}
                                    }
                                    p { class: "text-xs text-stone-500 mb-3",
                                        {t(&lang, "folder-menu-delete-warning")}
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
                                                let _ = db().delete_folder(&folder_id_for_delete);
                                                if (app.selected_folder_id)().as_deref() == Some(folder_id_for_delete.as_str()) {
                                                    app.selected_folder_id.set(None);
                                                }
                                                app.folders_version.set((app.folders_version)() + 1);
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
                                button {
                                    class: kit::MENU_ITEM,
                                    onclick: move |_| {
                                        show_actions.set(false);
                                        creating_sub.set(true);
                                    },
                                    IconFolderPlus { size: 16 }
                                    {t(&lang, "folder-menu-subtheme")}
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
            if creating_sub() {
                div {
                    class: "flex items-center gap-1 bg-stone-100 rounded-xl pl-3 pr-1 py-1 ml-8 my-1",
                    style: "animation: popIn 0.16s ease-out;",
                    input {
                        class: "flex-1 min-w-0 bg-transparent text-sm outline-none py-1.5 text-stone-900 placeholder-stone-400",
                        placeholder: t(&lang, "sidebar-subfolder-placeholder"),
                        value: "{sub_name}",
                        oninput: move |evt| sub_name.set(evt.value()),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Escape {
                                creating_sub.set(false);
                            }
                        },
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
                        class: "w-10 h-10 flex items-center justify-center rounded-lg transition-colors duration-150",
                        class: if sub_name().trim().is_empty() {
                            "text-stone-300"
                        } else {
                            "text-ios-orange-dark bg-ios-orange-50 active:opacity-70 hover:opacity-80"
                        },
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
                        IconCheck { size: 16 }
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
