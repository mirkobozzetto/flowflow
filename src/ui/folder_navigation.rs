//! Shared navigation controls for the sidebar and the top-bar theme picker.
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{IconMagnifyingGlass, IconX};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::collections::HashSet;

pub const CLOSED_FOLDERS_KEY: &str = "ui_closed_folders";

pub fn load_closed_folders(db: &Database) -> HashSet<String> {
    db.get_setting(CLOSED_FOLDERS_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn set_folder_expanded(
    mut app: AppState,
    db: &Database,
    id: &str,
    expanded: bool,
) {
    let mut closed = app.collapsed_folders.peek().clone();
    if expanded {
        closed.remove(id);
    } else {
        closed.insert(id.to_string());
    }
    // Only closed ids are stored: future folders start open, and folding a
    // parent never changes the saved choices for any of its descendants.
    let result = serde_json::to_string(&closed)
        .map_err(|e| e.to_string())
        .and_then(|json| db.set_setting(CLOSED_FOLDERS_KEY, &json));
    match result {
        Ok(()) => app.collapsed_folders.set(closed),
        Err(e) => eprintln!("[folders] save navigation state: {e}"),
    }
}

#[component]
pub fn FolderSearch(
    query: Signal<String>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    rsx! {
        div { class: "folder-search flex items-center gap-2 min-h-[44px] bg-stone-100 border border-stone-300 rounded-xl pl-3 pr-1 focus-within:border-stone-500",
            span { class: "text-stone-500", "aria-hidden": "true",
                IconMagnifyingGlass { size: 18 }
            }
            input {
                class: "flex-1 min-w-0 bg-transparent py-2.5 text-base text-stone-900 placeholder-stone-500 outline-none",
                r#type: "search",
                autocomplete: "off",
                placeholder: t(&lang, "folder-search-placeholder"),
                "aria-label": t(&lang, "folder-search-placeholder"),
                value: "{query}",
                oninput: move |evt| query.set(evt.value()),
                onkeydown: move |evt| onkeydown.call(evt),
            }
            if !query().is_empty() {
                button {
                    class: "w-11 h-11 shrink-0 flex items-center justify-center rounded-full text-stone-500 hover:bg-stone-200 focus-visible:outline-stone-500",
                    "aria-label": t(&lang, "folder-search-clear"),
                    // Keep the keyboard open when clearing the search.
                    onmousedown: move |evt| evt.prevent_default(),
                    onclick: move |_| query.set(String::new()),
                    IconX { size: 16 }
                }
            }
        }
    }
}
