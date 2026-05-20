use crate::db::Database;
use crate::ui::icons::*;
use crate::ui::{AppState, SidebarTab, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn TopBar() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let is_detail = matches!((app.view)(), View::NoteDetail { .. });
    let is_chat = matches!((app.view)(), View::Chat { .. });
    let is_settings = matches!((app.view)(), View::Settings);
    let is_inner = is_detail || is_chat || is_settings;

    let title = match (app.view)() {
        View::NotesList => match (app.selected_folder_id)() {
            Some(ref fid) => db()
                .get_folder(fid)
                .ok()
                .flatten()
                .map(|f| f.name)
                .unwrap_or_else(|| "Thème".to_string()),
            None => "Toutes mes notes".to_string(),
        },
        View::NoteDetail { .. } => match (app.detail_folder_id)() {
            Some(ref fid) => db()
                .get_folder(fid)
                .ok()
                .flatten()
                .map(|f| f.name)
                .unwrap_or_else(|| "Toutes les notes".to_string()),
            None => "Toutes les notes".to_string(),
        },
        View::Chat { .. } => match (app.chat_scope_folder_id)() {
            Some(ref fid) => db()
                .get_folder(fid)
                .ok()
                .flatten()
                .map(|f| f.name)
                .unwrap_or_else(|| "Toutes les notes".to_string()),
            None => "Toutes les notes".to_string(),
        },
        View::Settings => "Réglages".to_string(),
    };

    rsx! {
        div { class: "flex items-center px-4 py-3 bg-warm-white border-b border-stone-200 sticky top-0 z-30 gap-3 min-h-[44px]",
            if is_inner {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        app.sliding_out.set(true);
                        let target = (app.previous_view)()
                            .unwrap_or(View::NotesList);
                        spawn(async move {
                            futures_timer::Delay::new(std::time::Duration::from_millis(150)).await;
                            app.sliding_out.set(false);
                            app.previous_view.set(None);
                            app.view.set(target);
                        });
                    },
                    IconArrowLeft { size: 22 }
                }
            } else {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        app.sidebar_tab.set(SidebarTab::Notes);
                        app.sidebar_open.set(true);
                    },
                    IconList { size: 22 }
                }
            }
            if is_detail || is_chat || !is_inner {
                button {
                    class: "flex-1 text-left flex items-center gap-1.5 active:opacity-70 transition-opacity duration-150",
                    onclick: move |_| {
                        let cur = (app.show_folder_picker)();
                        app.show_folder_picker.set(!cur);
                    },
                    span { class: "text-lg font-semibold text-stone-900", "{title}" }
                    span {
                        class: if (app.show_folder_picker)() {
                            "inline-block w-1.5 h-1.5 border-r-2 border-b-2 border-stone-400 transition-transform duration-150 -rotate-[135deg]"
                        } else {
                            "inline-block w-1.5 h-1.5 border-r-2 border-b-2 border-stone-400 transition-transform duration-150 rotate-45"
                        },
                    }
                }
            } else {
                span { class: "text-lg font-semibold text-stone-900 flex-1", "{title}" }
            }
            if is_detail {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        let cur = (app.show_note_menu)();
                        app.show_note_menu.set(!cur);
                    },
                    IconDotsThreeVertical { size: 22 }
                }
            } else if is_chat {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        let cur = (app.show_chat_menu)();
                        app.show_chat_menu.set(!cur);
                    },
                    IconDotsThreeVertical { size: 22 }
                }
            } else if !is_inner {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-ios-orange-dark",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        app.sidebar_tab.set(SidebarTab::Chats);
                        app.sidebar_open.set(true);
                    },
                    IconChatAi { size: 28 }
                }
            }
        }
    }
}
