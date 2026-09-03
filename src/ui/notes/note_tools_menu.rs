use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::tools_menu::{
    ConnectorsSection, LEAD_ICON, ROW, ROW_SUB, ROW_TITLE, SECTION, SEP,
};
use crate::ui::hooks::swipe::use_sheet_dismiss;
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

// The note "+" hub: same shell, palette and connectors as the chat composer, scoped to a
// note. Bottom-sheet (drag-to-dismiss) on mobile, popover above the "+" on desktop.
// Connector taps route to Settings > Connections.
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
                NoteToolsMenuBody {}
            }
        }
    } else {
        rsx! {
            div { class: "fixed inset-0 z-30", onclick: close }
            div { class: "absolute bottom-full left-0 mb-2 w-80 z-40 bg-warm-white border border-stone-200 rounded-xl shadow-menu p-2 popover-pop",
                NoteToolsMenuBody {}
            }
        }
    }
}

#[component]
fn NoteToolsMenuBody() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    // Same source as the chat palette. The difference is the tap: a note has no composer,
    // so the agent runs on the note's own text instead of writing a launch command.
    let agents = crate::application::agent_activation::palette_entries(&db());

    rsx! {
        if !agents.is_empty() {
            div { class: SECTION, {t(&lang, "chat-tools-section-agents")} }
            for agent in agents {
                button {
                    class: ROW,
                    key: "{agent.id}",
                    onclick: {
                        let agent = agent.clone();
                        move |_| {
                            app.show_note_tools_menu.set(false);
                            app.pending_note_agent.set(Some(agent.clone()));
                        }
                    },
                    span { class: LEAD_ICON, IconChatAi { size: 22 } }
                    span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                        span { class: ROW_TITLE, "{agent.name}" }
                        span { class: ROW_SUB, "{agent.description}" }
                    }
                }
            }
            div { class: SEP }
        }
        ConnectorsSection {}
    }
}
