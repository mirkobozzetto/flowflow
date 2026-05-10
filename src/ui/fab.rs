use crate::ui::icons::*;
use crate::ui::{AppState, View};
use dioxus::prelude::*;

#[component]
pub fn FloatingActionButton() -> Element {
    let mut app: AppState = use_context();

    rsx! {
        div { class: "fixed bottom-6 right-4 z-20",
            button {
                class: "w-20 h-20 rounded-full bg-white flex items-center justify-center shadow-xl shadow-gray-400/40",
                onclick: move |_| {
                    app.view.set(View::NoteDetail { note_id: String::new() });
                },
                div { class: "text-gray-700",
                    IconNewNote { size: 40 }
                }
            }
        }
    }
}
