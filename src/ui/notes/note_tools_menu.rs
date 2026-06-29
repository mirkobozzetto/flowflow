use crate::ui::chat::ConnectorsSection;
use crate::ui::hooks::swipe::use_sheet_dismiss;
use crate::ui::AppState;
use dioxus::prelude::*;

// The note "+" hub: same shell and connectors as the chat composer, scoped to a
// note. Bottom-sheet (drag-to-dismiss) on mobile, popover above the "+" on
// desktop. Connector taps route to Settings > Connections.
#[component]
pub fn NoteToolsMenu() -> Element {
    let mut app: AppState = use_context();

    use_sheet_dismiss(
        "note-tools-sheet",
        "note-tools-backdrop",
        48.0,
        0.3,
        move || app.show_note_tools_menu.set(false),
    );

    let close = move |_| app.show_note_tools_menu.set(false);

    if cfg!(target_os = "ios") {
        rsx! {
            div {
                id: "note-tools-backdrop",
                class: "fixed inset-0 z-30 bg-black/10 backdrop-fade",
                onclick: close,
            }
            div {
                id: "note-tools-sheet",
                class: "fixed bottom-0 left-0 right-0 z-40 bg-warm-white rounded-t-2xl px-2 pt-2 sheet-pop",
                style: "padding-bottom: calc(1.5rem + env(safe-area-inset-bottom));",
                div { class: "w-9 h-1 rounded-full bg-stone-300 mx-auto mt-1 mb-3" }
                ConnectorsSection {}
            }
        }
    } else {
        rsx! {
            div { class: "fixed inset-0 z-30", onclick: close }
            div { class: "absolute bottom-full left-0 mb-2 w-80 z-40 bg-warm-white border border-stone-200 rounded-xl shadow-lg p-2 popover-pop",
                ConnectorsSection {}
            }
        }
    }
}
