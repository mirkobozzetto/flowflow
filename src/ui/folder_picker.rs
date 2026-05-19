use crate::db::Database;
use crate::models::Folder;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn FolderPicker(selected: Signal<Option<String>>) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();

    let all_folders: Memo<Vec<Folder>> = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_all_folders().unwrap_or_default()
    });

    rsx! {
        div { class: "absolute left-0 right-0 top-0 z-20 bg-warm-white border-b border-stone-200 overflow-hidden shadow-md animate-[slideDown_150ms_ease-out]",
            button {
                class: if selected().is_none() {
                    "w-full text-left px-3 py-2.5 text-sm text-ios-orange-dark font-medium bg-ios-orange-50 border-b border-stone-100"
                } else {
                    "w-full text-left px-3 py-2.5 text-sm text-stone-500 border-b border-stone-100 active:bg-stone-50 transition-colors duration-150"
                },
                onclick: move |_| {
                    selected.set(None);
                    app.show_folder_picker.set(false);
                },
                "Global"
            }
            for folder in all_folders() {
                {
                    let fid = folder.id.clone();
                    let fname = folder.name.clone();
                    let is_current = selected() == Some(fid.clone());
                    rsx! {
                        button {
                            class: "w-full text-left px-3 py-2.5 text-sm active:bg-stone-50 transition-colors duration-150",
                            class: if is_current { "text-ios-orange-dark font-medium bg-ios-orange-50" } else { "text-stone-900" },
                            onclick: move |_| {
                                selected.set(Some(fid.clone()));
                                app.show_folder_picker.set(false);
                            },
                            "{fname}"
                        }
                    }
                }
            }
        }
    }
}
