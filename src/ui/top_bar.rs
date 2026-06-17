use crate::db::Database;
use crate::services::i18n::t;
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
    let is_settings =
        matches!((app.view)(), View::Settings | View::SettingsSection(_));
    let is_sync_pairing = matches!((app.view)(), View::SyncPairing);
    let is_inner = is_detail || is_chat || is_settings || is_sync_pairing;
    let chat_from_note = is_chat
        && matches!((app.previous_view)(), Some(View::NoteDetail { .. }));
    let show_back = (is_inner && !is_chat) || chat_from_note;
    let lang = (app.current_lang)();

    let title = match (app.view)() {
        View::NotesList => match (app.selected_folder_id)() {
            Some(ref fid) => db()
                .get_folder(fid)
                .ok()
                .flatten()
                .map(|f| f.name)
                .unwrap_or_else(|| t(&lang, "top-bar-folder-fallback")),
            None => t(&lang, "sidebar-all-notes"),
        },
        View::NoteDetail { .. } => match (app.detail_folder_id)() {
            Some(ref fid) => db()
                .get_folder(fid)
                .ok()
                .flatten()
                .map(|f| f.name)
                .unwrap_or_else(|| t(&lang, "top-bar-all-notes")),
            None => t(&lang, "top-bar-all-notes"),
        },
        View::Chat { .. } => match (app.chat_scope_folder_id)() {
            Some(ref fid) => db()
                .get_folder(fid)
                .ok()
                .flatten()
                .map(|f| f.name)
                .unwrap_or_else(|| t(&lang, "top-bar-all-notes")),
            None => t(&lang, "top-bar-all-notes"),
        },
        View::Settings => t(&lang, "sidebar-settings"),
        View::SettingsSection(section) => t(&lang, section.title_key()),
        View::SyncPairing => t(&lang, "sync-pairing-title"),
    };

    rsx! {
        div { class: "flex items-center px-4 py-3 bg-warm-white border-b border-stone-200 sticky top-0 z-30 gap-3 min-h-[44px]",
            if show_back {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700 hover:text-stone-900 transition-colors duration-150",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        let target = (app.previous_view)()
                            .unwrap_or(View::NotesList);
                        if cfg!(target_os = "macos")
                            || matches!(
                                target,
                                View::Chat { .. } | View::NoteDetail { .. }
                            )
                        {
                            app.previous_view.set(None);
                            app.view.set(target);
                            return;
                        }
                        app.sliding_out.set(true);
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
                    class: "relative min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700 lg:hidden",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        app.sidebar_tab.set(if is_chat {
                            SidebarTab::Chats
                        } else {
                            SidebarTab::Notes
                        });
                        app.sidebar_open.set(true);
                    },
                    IconList { size: 22 }
                    if (app.transcription_done_badge)() > 0 {
                        span { class: "absolute top-2 right-2 w-2 h-2 rounded-full bg-ios-orange" }
                    }
                }
            }
            if is_detail || is_chat || !is_inner {
                button {
                    class: "flex-1 text-left flex items-center gap-1.5 active:opacity-70 hover:opacity-70 transition-opacity duration-150",
                    onclick: move |_| {
                        let cur = (app.show_folder_picker)();
                        app.show_folder_picker.set(!cur);
                    },
                    span { class: "text-lg font-semibold text-stone-900", "{title}" }
                    span {
                        class: "inline-block w-1.5 h-1.5 border-r-2 border-b-2 border-stone-400 chevron-pivot",
                        class: if (app.show_folder_picker)() { "-rotate-[135deg]" } else { "rotate-45" },
                    }
                }
            } else {
                span { class: "text-lg font-semibold text-stone-900 flex-1", "{title}" }
            }
            if is_detail {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700 hover:text-stone-900 transition-colors duration-150",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        let cur = (app.show_note_menu)();
                        app.show_note_menu.set(!cur);
                    },
                    IconDotsThreeVertical { size: 22 }
                }
            } else if is_chat {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700 hover:text-stone-900 transition-colors duration-150",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        let cur = (app.show_chat_menu)();
                        app.show_chat_menu.set(!cur);
                    },
                    IconDotsThreeVertical { size: 22 }
                }
            } else if !is_inner {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-ios-orange-dark hover:opacity-70 transition-opacity duration-150",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        app.sidebar_tab.set(SidebarTab::Chats);
                        app.chat_scope_folder_id.set(None);
                        app.previous_view.set(Some(View::NotesList));
                        app.view.set(View::Chat { conversation_id: None });
                    },
                    IconChatAi { size: 28 }
                }
            }
        }
    }
}
