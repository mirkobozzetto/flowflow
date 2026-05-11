use crate::db::Database;
use crate::ui::icons::*;
use crate::ui::{AppState, View};
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
                .unwrap_or_else(|| "Dossier".to_string()),
            None => "Toutes mes notes".to_string(),
        },
        View::NoteDetail { .. } => "Note".to_string(),
        View::Chat { .. } => "Chat".to_string(),
        View::Settings => "Réglages".to_string(),
    };

    rsx! {
        div { class: "flex items-center px-4 py-3 bg-white border-b border-gray-200 sticky top-0 z-30 gap-3 min-h-[44px]",
            if is_inner {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-700",
                    onclick: move |_| {
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
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-700",
                    onclick: move |_| app.sidebar_open.set(true),
                    IconList { size: 22 }
                }
            }
            span { class: "text-lg font-semibold text-gray-900 flex-1", "{title}" }
            if is_detail {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-700",
                    onclick: move |_| {
                        let cur = (app.show_note_menu)();
                        app.show_note_menu.set(!cur);
                    },
                    IconDotsThreeVertical { size: 22 }
                }
            } else if !is_inner {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-900",
                    onclick: move |_| {
                        app.view.set(View::Chat { conversation_id: None });
                    },
                    IconChatAi { size: 28 }
                }
            }
        }
    }
}
