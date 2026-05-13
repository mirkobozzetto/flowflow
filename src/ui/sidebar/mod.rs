mod conversations;
mod folders;

pub use conversations::*;
pub use folders::*;

use crate::db::Database;
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
            class: "fixed left-0 top-0 w-[85vw] max-w-[340px] h-full bg-warm-white z-50 flex flex-col border-r border-stone-200 transition-transform duration-200",
            class: if is_open { "translate-x-0" } else { "-translate-x-full" },
            onclick: move |evt| evt.stop_propagation(),

            div { class: "flex border-b border-stone-200",
                button {
                    class: if active_tab() == SidebarTab::Notes {
                        "flex-1 py-3 text-sm font-semibold text-ios-orange-dark border-b-2 border-ios-orange-dark"
                    } else {
                        "flex-1 py-3 text-sm font-medium text-stone-400"
                    },
                    onclick: move |_| active_tab.set(SidebarTab::Notes),
                    div { class: "flex items-center justify-center gap-1.5",
                        IconNotePencil { size: 16 }
                        "Notes"
                    }
                }
                button {
                    class: if active_tab() == SidebarTab::Chats {
                        "flex-1 py-3 text-sm font-semibold text-ios-orange-dark border-b-2 border-ios-orange-dark"
                    } else {
                        "flex-1 py-3 text-sm font-medium text-stone-400"
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
                                class: "flex items-center gap-2.5 w-full px-2 py-3 text-base text-stone-900 font-semibold rounded-lg min-h-[44px]",
                                onclick: move |_| {
                                    app.selected_folder_id.set(None);
                                    app.view.set(View::NotesList);
                                    app.sidebar_open.set(false);
                                },
                                IconNotebook { size: 20 }
                                "Toutes mes notes"
                            }
                        }
                        div { class: "h-px bg-stone-200 mb-2" }
                        FolderSection {}
                    },
                    SidebarTab::Chats => rsx! {
                        ConversationSection {}
                    },
                }
            }

            div { class: "border-t border-stone-200 p-4",
                button {
                    class: "flex items-center gap-2.5 w-full px-2 py-3 text-sm text-stone-500 rounded-lg min-h-[44px]",
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
