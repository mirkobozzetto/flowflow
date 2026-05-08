use crate::db::Database;
use crate::models::Folder;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn TopBar() -> Element {
    let mut app: AppState = use_context();
    let is_detail = matches!((app.view)(), View::NoteDetail { .. });

    let title = match (app.view)() {
        View::NotesList => match (app.selected_folder_id)() {
            Some(_) => "Dossier".to_string(),
            None => "Toutes mes notes".to_string(),
        },
        View::NoteDetail { .. } => "Note".to_string(),
    };

    rsx! {
        div { class: "flex items-center px-4 py-3 bg-white border-b border-gray-200 sticky top-0 z-30 gap-3 min-h-[44px]",
            if is_detail {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center",
                    onclick: move |_| app.view.set(View::NotesList),
                    div { class: "w-2.5 h-2.5 border-l-2 border-b-2 border-gray-700 rotate-45" }
                }
            } else {
                button {
                    class: "min-w-[44px] min-h-[44px] flex flex-col items-center justify-center gap-1",
                    onclick: move |_| app.sidebar_open.set(true),
                    div { class: "w-5 h-0.5 bg-gray-700" }
                    div { class: "w-5 h-0.5 bg-gray-700" }
                    div { class: "w-5 h-0.5 bg-gray-700" }
                }
            }
            span { class: "text-lg font-semibold text-gray-900 flex-1", "{title}" }
        }
    }
}

#[component]
pub fn SidebarOverlay() -> Element {
    let mut app: AppState = use_context();
    let is_open = (app.sidebar_open)();

    rsx! {
        div {
            class: "fixed inset-0 bg-black/30 z-40 transition-opacity duration-200",
            class: if is_open { "opacity-100" } else { "opacity-0 pointer-events-none" },
            onclick: move |_| app.sidebar_open.set(false),
        }
        div {
            class: "fixed left-0 top-0 w-[280px] h-full bg-white z-50 p-4 overflow-y-auto border-r border-gray-200 transition-transform duration-200",
            class: if is_open { "translate-x-0" } else { "-translate-x-full" },
            onclick: move |evt| evt.stop_propagation(),

            div { class: "py-2 pb-4",
                button {
                    class: "flex items-center gap-2.5 w-full px-2 py-3 text-base text-gray-900 font-semibold rounded-lg min-h-[44px]",
                    onclick: move |_| {
                        app.selected_folder_id.set(None);
                        app.view.set(View::NotesList);
                        app.sidebar_open.set(false);
                    },
                    "Toutes mes notes"
                }
            }

            div { class: "h-px bg-gray-200 mb-2" }

            FolderTree {}
        }
    }
}

#[component]
fn FolderTree() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let folders = use_signal(|| db().list_root_folders().unwrap_or_default());

    if folders().is_empty() {
        return rsx! {
            p { class: "text-xs text-gray-400 px-2 py-3", "Aucun dossier" }
        };
    }

    rsx! {
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
    let children =
        use_signal(|| db().list_subfolders(&folder.id).unwrap_or_default());
    let has_children = !children().is_empty();
    let folder_id = folder.id.clone();

    let is_selected =
        (app.selected_folder_id)().as_deref() == Some(folder_id.as_str());

    let margin = format!("margin-left: {}px", depth * 16);

    rsx! {
        div { style: "{margin}",
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
                    class: "flex-1 text-left px-2 py-2.5 text-sm text-gray-900 rounded-lg min-h-[44px]",
                    class: if is_selected { "bg-gray-100" },
                    onclick: move |_| {
                        app.selected_folder_id.set(Some(folder_id.clone()));
                        app.view.set(View::NotesList);
                        app.sidebar_open.set(false);
                    },
                    "{folder.name}"
                }
            }
            if expanded() {
                for child in children() {
                    FolderItem { folder: child, depth: depth + 1 }
                }
            }
        }
    }
}

#[component]
pub fn FloatingActionButton() -> Element {
    let mut app: AppState = use_context();

    rsx! {
        div { class: "fixed bottom-6 right-4 z-20",
            button {
                class: "w-14 h-14 rounded-full bg-ios-green flex items-center justify-center shadow-lg shadow-ios-green/40",
                onclick: move |_| {
                    app.view.set(View::NoteDetail { note_id: String::new() });
                },
                div { class: "relative w-5 h-5",
                    div { class: "absolute top-1/2 left-0 w-full h-0.5 bg-white -translate-y-1/2" }
                    div { class: "absolute left-1/2 top-0 w-0.5 h-full bg-white -translate-x-1/2" }
                }
            }
        }
    }
}
