use crate::db::Database;
use crate::models::{Folder, NewFolder, UpdateFolder};
use crate::ui::icons::*;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
enum SidebarTab {
    Notes,
    Chats,
}

#[component]
pub fn SidebarOverlay() -> Element {
    let mut app: AppState = use_context();
    let _db: Signal<Arc<Database>> = use_context();
    let is_open = (app.sidebar_open)();
    let mut active_tab = use_signal(|| SidebarTab::Notes);

    rsx! {
        div {
            class: "fixed inset-0 bg-black/30 z-40 transition-opacity duration-200",
            class: if is_open { "opacity-100" } else { "opacity-0 pointer-events-none" },
            onclick: move |_| app.sidebar_open.set(false),
        }
        div {
            class: "fixed left-0 top-0 w-[85vw] max-w-[340px] h-full bg-white z-50 flex flex-col border-r border-gray-200 transition-transform duration-200",
            class: if is_open { "translate-x-0" } else { "-translate-x-full" },
            onclick: move |evt| evt.stop_propagation(),

            div { class: "flex border-b border-gray-200",
                button {
                    class: if active_tab() == SidebarTab::Notes {
                        "flex-1 py-3 text-sm font-semibold text-ios-blue border-b-2 border-ios-blue"
                    } else {
                        "flex-1 py-3 text-sm font-medium text-gray-400"
                    },
                    onclick: move |_| active_tab.set(SidebarTab::Notes),
                    div { class: "flex items-center justify-center gap-1.5",
                        IconNotePencil { size: 16 }
                        "Notes"
                    }
                }
                button {
                    class: if active_tab() == SidebarTab::Chats {
                        "flex-1 py-3 text-sm font-semibold text-ios-blue border-b-2 border-ios-blue"
                    } else {
                        "flex-1 py-3 text-sm font-medium text-gray-400"
                    },
                    onclick: move |_| active_tab.set(SidebarTab::Chats),
                    div { class: "flex items-center justify-center gap-1.5",
                        IconChats { size: 16 }
                        "Chats"
                    }
                }
            }

            div { class: "flex-1 overflow-y-auto p-4",
                match active_tab() {
                    SidebarTab::Notes => rsx! {
                        div { class: "py-2 pb-4",
                            button {
                                class: "flex items-center gap-2.5 w-full px-2 py-3 text-base text-gray-900 font-semibold rounded-lg min-h-[44px]",
                                onclick: move |_| {
                                    app.selected_folder_id.set(None);
                                    app.view.set(View::NotesList);
                                    app.sidebar_open.set(false);
                                },
                                IconNotebook { size: 20 }
                                "Toutes mes notes"
                            }
                        }
                        div { class: "h-px bg-gray-200 mb-2" }
                        FolderSection {}
                    },
                    SidebarTab::Chats => rsx! {
                        ConversationSection {}
                    },
                }
            }

            div { class: "border-t border-gray-200 p-4",
                button {
                    class: "flex items-center gap-2.5 w-full px-2 py-3 text-sm text-gray-500 rounded-lg min-h-[44px]",
                    onclick: move |_| {
                        app.view.set(View::Settings);
                        app.sidebar_open.set(false);
                    },
                    IconGear { size: 18 }
                    "Réglages"
                }
            }
        }
    }
}

#[component]
fn ConversationSection() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let mut version = use_signal(|| 0u32);

    let conversations = use_memo(move || {
        let _v = version();
        db().list_conversations().unwrap_or_default()
    });

    rsx! {
        button {
            class: "flex items-center gap-2 w-full px-2 py-3 text-sm font-medium text-ios-blue rounded-lg min-h-[44px] mb-2",
            onclick: move |_| {
                app.view.set(View::Chat { conversation_id: None });
                app.sidebar_open.set(false);
            },
            IconPlus { size: 16 }
            "Nouvelle conversation"
        }
        if conversations().is_empty() {
            p { class: "text-xs text-gray-400 px-2 py-3", "Aucune conversation" }
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

    let conv_id_nav = conv.id.clone();
    let conv_id_rename = conv.id.clone();
    let conv_id_rename2 = conv.id.clone();
    let conv_id_del = conv.id.clone();
    let title = if conv.title.is_empty() {
        "Sans titre".to_string()
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
                    class: "flex-1 text-sm border border-gray-200 rounded-lg px-2 py-1.5 outline-none",
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
                    class: "w-8 h-8 flex items-center justify-center rounded-lg bg-ios-blue text-white",
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
                    class: "w-9 h-9 flex items-center justify-center text-gray-400",
                    onclick: move |_| editing.set(false),
                    IconX { size: 16 }
                }
            }
        } else if confirm_delete() {
            div { class: "flex items-center gap-2 py-2 px-2",
                span { class: "flex-1 text-sm text-gray-600", "Supprimer ?" }
                button {
                    class: "px-3 py-1 rounded-lg bg-ios-red text-white text-xs font-medium",
                    onclick: move |_| {
                        let _ = db().delete_conversation(&conv_id_del);
                        version.set(version() + 1);
                    },
                    "Oui"
                }
                button {
                    class: "px-3 py-1 rounded-lg bg-gray-200 text-gray-600 text-xs",
                    onclick: move |_| confirm_delete.set(false),
                    "Non"
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
                    p { class: "text-sm text-gray-900 line-clamp-1", "{title}" }
                    p { class: "text-[10px] text-gray-400 mt-0.5", "{date}" }
                }
                button {
                    class: "w-9 h-9 flex items-center justify-center text-gray-400",
                    onclick: move |_| show_actions.set(!show_actions()),
                    IconDotsThree { size: 20 }
                }
            }
            if show_actions() {
                div { class: "flex items-center gap-0 px-2 py-1 ml-2",
                    button {
                        class: "w-10 h-10 flex items-center justify-center text-gray-500",
                        onclick: move |_| {
                            show_actions.set(false);
                            editing.set(true);
                        },
                        IconPencil { size: 18 }
                    }
                    button {
                        class: "w-10 h-10 flex items-center justify-center text-gray-400",
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

#[component]
fn FolderSection() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let mut creating = use_signal(|| false);
    let mut new_name = use_signal(String::new);

    let folders = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_root_folders().unwrap_or_default()
    });

    rsx! {
        div { class: "flex items-center justify-between px-2 mb-2",
            span { class: "text-xs font-medium text-gray-400 uppercase tracking-wide", "Dossiers" }
            button {
                class: "w-9 h-9 flex items-center justify-center rounded-full text-gray-500",
                onclick: move |_| creating.set(!creating()),
                IconPlus { size: 18 }
            }
        }
        if creating() {
            div { class: "flex items-center gap-2 px-2 mb-2",
                input {
                    class: "flex-1 text-sm border border-gray-200 rounded-lg px-2 py-1.5 outline-none",
                    placeholder: "Nom du dossier",
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
                    class: "w-9 h-9 flex items-center justify-center rounded-lg bg-ios-blue text-white",
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
            p { class: "text-xs text-gray-400 px-2 py-3", "Aucun dossier" }
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
                        class: "flex-1 text-sm border border-gray-200 rounded-lg px-2 py-1.5 outline-none",
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
                        class: "w-8 h-8 flex items-center justify-center rounded-lg bg-ios-blue text-white text-xs font-medium",
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
                        class: "w-9 h-9 flex items-center justify-center text-gray-400",
                        onclick: move |_| editing.set(false),
                        IconX { size: 16 }
                    }
                }
            } else if confirm_delete() {
                div { class: "flex items-center gap-2 py-2 px-2",
                    span { class: "flex-1 text-sm text-gray-600", "Supprimer ?" }
                    button {
                        class: "px-3 py-1 rounded-lg bg-ios-red text-white text-xs font-medium",
                        onclick: move |_| {
                            let _ = db().delete_folder(&folder_id_for_delete);
                            if (app.selected_folder_id)().as_deref() == Some(folder_id_for_delete.as_str()) {
                                app.selected_folder_id.set(None);
                            }
                            app.folders_version.set((app.folders_version)() + 1);
                        },
                        "Oui"
                    }
                    button {
                        class: "px-3 py-1 rounded-lg bg-gray-200 text-gray-600 text-xs",
                        onclick: move |_| confirm_delete.set(false),
                        "Non"
                    }
                }
            } else {
                div { class: "flex items-center",
                    if has_children {
                        button {
                            class: "min-w-[32px] min-h-[44px] flex items-center justify-center",
                            onclick: move |_| expanded.set(!expanded()),
                            div {
                                class: "w-1.5 h-1.5 border-r-2 border-b-2 border-gray-400 transition-transform duration-150",
                                class: if expanded() { "rotate-45" } else { "-rotate-45" },
                            }
                        }
                    } else {
                        div { class: "w-8 min-w-[32px]" }
                    }
                    button {
                        class: "flex-1 flex items-center gap-2 text-left px-2 py-2.5 text-sm text-gray-900 rounded-lg min-h-[44px]",
                        class: if is_selected { "bg-gray-100" },
                        onclick: move |_| {
                            app.selected_folder_id.set(Some(folder_id_nav.clone()));
                            app.view.set(View::NotesList);
                            app.sidebar_open.set(false);
                        },
                        IconFolder { size: 16 }
                        "{folder.name}"
                    }
                    button {
                        class: "w-9 h-9 flex items-center justify-center text-gray-400",
                        onclick: move |_| show_actions.set(!show_actions()),
                        IconDotsThree { size: 20 }
                    }
                }
                if show_actions() {
                    div { class: "flex items-center gap-0 px-2 py-1 ml-8",
                        button {
                            class: "w-10 h-10 flex items-center justify-center text-gray-500",
                            onclick: move |_| {
                                show_actions.set(false);
                                editing.set(true);
                            },
                            IconPencil { size: 18 }
                        }
                        button {
                            class: "w-10 h-10 flex items-center justify-center text-gray-500",
                            onclick: move |_| {
                                show_actions.set(false);
                                creating_sub.set(true);
                            },
                            IconFolderPlus { size: 18 }
                        }
                        button {
                            class: "w-10 h-10 flex items-center justify-center text-gray-400",
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
                        class: "flex-1 text-sm border border-gray-200 rounded-lg px-2 py-1.5 outline-none",
                        placeholder: "Sous-dossier",
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
                        class: "w-8 h-8 flex items-center justify-center rounded-lg bg-ios-blue text-white text-xs font-medium",
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
