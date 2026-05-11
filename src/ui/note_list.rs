use crate::db::Database;
use crate::ui::icons::*;
use crate::ui::note_card::NoteCard;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn NotesList() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();

    let notes = use_memo(move || {
        let _v = (app.notes_version)();
        let db = db();
        let all = match (app.selected_folder_id)() {
            Some(fid) => db.list_notes_in_folder(&fid).unwrap_or_default(),
            None => db.list_notes().unwrap_or_default(),
        };
        let q = (app.search_query)().to_lowercase();
        if q.is_empty() {
            return all;
        }
        all.into_iter()
            .filter(|n| {
                n.title.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || n.content.to_lowercase().contains(&q)
                    || n.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    });

    let has_query = !(app.search_query)().is_empty();

    rsx! {
        div { class: "mb-3",
            div { class: "relative",
                div { class: "absolute left-3 top-1/2 -translate-y-1/2 text-gray-400",
                    IconMagnifyingGlass { size: 16 }
                }
                input {
                    class: "w-full bg-white border border-gray-200 rounded-xl pl-9 pr-9 py-2.5 text-sm outline-none text-gray-900 placeholder-gray-400",
                    placeholder: "Chercher mes notes...",
                    value: "{app.search_query}",
                    oninput: move |evt| app.search_query.set(evt.value()),
                }
                if has_query {
                    button {
                        class: "absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 active:text-gray-600 p-1",
                        onclick: move |_| app.search_query.set(String::new()),
                        IconX { size: 14 }
                    }
                }
            }
        }
        if notes().is_empty() {
            div { class: "flex-1 flex flex-col items-center justify-center gap-2 h-[60vh]",
                if has_query {
                    p { class: "text-lg text-gray-400", "Aucun résultat" }
                    p { class: "text-sm text-gray-400", "Essayez un autre terme" }
                } else {
                    p { class: "text-lg text-gray-400", "Aucune note" }
                    p { class: "text-sm text-gray-400", "Appuyez sur + pour commencer" }
                }
            }
        } else {
            div { class: "pb-32",
                for note in notes() {
                    NoteCard { note: note }
                }
            }
        }
    }
}
