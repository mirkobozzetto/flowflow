use crate::db::Database;
use crate::models::Folder;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn FolderPicker(selected: Signal<Option<String>>) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut show = use_signal(|| false);

    let all_folders: Memo<Vec<Folder>> = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_all_folders().unwrap_or_default()
    });

    let display = match selected() {
        Some(ref fid) => all_folders()
            .iter()
            .find(|f| f.id == *fid)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "Dossier".to_string()),
        None => "Aucun dossier".to_string(),
    };

    rsx! {
        div { class: "mb-3",
            button {
                class: "inline-flex items-center px-3 py-1.5 rounded-full border border-gray-200 text-xs text-gray-600",
                onclick: move |_| show.set(!show()),
                "{display}"
            }
            if show() {
                div { class: "mt-1 border border-gray-200 rounded-xl bg-white overflow-hidden",
                    button {
                        class: "w-full text-left px-3 py-2.5 text-sm text-gray-500 border-b border-gray-100",
                        onclick: move |_| {
                            selected.set(None);
                            show.set(false);
                        },
                        "Aucun dossier"
                    }
                    for folder in all_folders() {
                        {
                            let fid = folder.id.clone();
                            let fname = folder.name.clone();
                            let is_current = selected() == Some(fid.clone());
                            rsx! {
                                button {
                                    class: "w-full text-left px-3 py-2.5 text-sm",
                                    class: if is_current { "text-ios-blue font-medium bg-blue-50" } else { "text-gray-900" },
                                    onclick: move |_| {
                                        selected.set(Some(fid.clone()));
                                        show.set(false);
                                    },
                                    "{fname}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
