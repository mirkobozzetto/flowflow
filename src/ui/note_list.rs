use crate::db::Database;
use crate::ui::note_card::NoteCard;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn NotesList() -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();

    let notes = use_memo(move || {
        let _v = (app.notes_version)();
        let db = db();
        match (app.selected_folder_id)() {
            Some(fid) => db.list_notes_in_folder(&fid).unwrap_or_default(),
            None => db.list_notes().unwrap_or_default(),
        }
    });

    rsx! {
        if notes().is_empty() {
            div { class: "flex-1 flex flex-col items-center justify-center gap-2 h-[60vh]",
                p { class: "text-lg text-gray-400", "Aucune note" }
                p { class: "text-sm text-gray-400", "Appuyez sur + pour commencer" }
            }
        } else {
            div {
                for note in notes() {
                    NoteCard { note: note }
                }
            }
        }
    }
}
