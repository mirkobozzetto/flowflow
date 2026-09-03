use crate::application::i18n::t;
use crate::domain::ChatScope;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::*;
use crate::ui::{AppState, SidebarTab, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn TopBar() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let is_detail = matches!((app.view)(), View::NoteDetail { .. });
    let is_thread = matches!((app.view)(), View::ThreadDetail { .. });
    let is_chat = matches!((app.view)(), View::Chat { .. });
    let is_settings =
        matches!((app.view)(), View::Settings | View::SettingsSection(_));
    let is_sync_pairing = matches!((app.view)(), View::SyncPairing);
    let is_shared = matches!((app.view)(), View::SharedView { .. });
    let is_inner = is_detail
        || is_thread
        || is_chat
        || is_settings
        || is_sync_pairing
        || is_shared;
    let chat_from_detail = is_chat
        && matches!(
            (app.previous_view)(),
            Some(View::NoteDetail { .. }) | Some(View::ThreadDetail { .. })
        );
    let show_back = (is_inner && !is_chat) || chat_from_detail;
    let lang = (app.current_lang)();

    let thread_title = |id: &str| {
        db().get_thread(id)
            .ok()
            .flatten()
            .map(|th| th.title)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| t(&lang, "thread-untitled"))
    };

    // Theme (folder) of the open thread, live: re-reads on theme change
    // (menu bumps notes_version) and on folder renames.
    let thread_theme = {
        let _ = (app.notes_version)();
        let _ = (app.folders_version)();
        match (app.view)() {
            View::ThreadDetail { ref thread_id } => db()
                .get_thread(thread_id)
                .ok()
                .flatten()
                .and_then(|th| th.folder_id)
                .and_then(|fid| db().get_folder(&fid).ok().flatten())
                .map(|f| f.name),
            _ => None,
        }
    };

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
        View::ThreadDetail { ref thread_id } => thread_title(thread_id),
        View::Chat { .. } => match (app.chat_scope)() {
            Some(ChatScope::Folder(ref fid)) => db()
                .get_folder(fid)
                .ok()
                .flatten()
                .map(|f| f.name)
                .unwrap_or_else(|| t(&lang, "top-bar-all-notes")),
            Some(ChatScope::Thread(ref tid)) => thread_title(tid),
            None => t(&lang, "top-bar-all-notes"),
        },
        View::SharedView { .. } => t(&lang, "share-open-title"),
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
                        // Note -> note back needs the NotesList bounce, else the
                        // mounted NoteDetail keeps the previous note's state.
                        if matches!((app.view)(), View::NoteDetail { .. })
                            && matches!(target, View::NoteDetail { .. })
                        {
                            app.previous_view.set(None);
                            app.view.set(View::NotesList);
                            spawn(async move {
                                futures_timer::Delay::new(std::time::Duration::from_millis(30)).await;
                                app.view.set(target);
                            });
                            return;
                        }
                        if cfg!(target_os = "macos")
                            || matches!(
                                target,
                                View::Chat { .. }
                                    | View::NoteDetail { .. }
                                    | View::ThreadDetail { .. }
                            )
                        {
                            app.previous_view.set(None);
                            app.view.set(target);
                            return;
                        }
                        app.sliding_out.set(true);
                        // spawn_forever: a scope-bound task cancelled by an unmount
                        // mid-delay would leave sliding_out stuck true (global freeze).
                        dioxus::core::spawn_forever(async move {
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
                    img {
                        src: asset!("/assets/flowflow-icon-300.png"),
                        class: "absolute bottom-1 right-1 w-2.5 h-2.5 object-contain",
                        alt: "",
                    }
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
                    span { class: "text-lg font-semibold tracking-[-0.01em] text-stone-900", "{title}" }
                    span {
                        class: "text-stone-400 transition-transform duration-150",
                        class: if (app.show_folder_picker)() { "rotate-90" } else { "rotate-0" },
                        IconCaretRight { size: 14 }
                    }
                }
            } else if is_thread {
                button {
                    class: "flex-1 min-w-0 text-left flex flex-col justify-center active:opacity-70 hover:opacity-70 transition-opacity duration-150",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        app.show_thread_theme.set(true);
                        app.show_thread_menu.set(true);
                    },
                    span { class: "text-lg font-semibold tracking-[-0.01em] text-stone-900 leading-tight truncate", "{title}" }
                    span {
                        class: "text-xs leading-tight truncate",
                        class: if thread_theme.is_some() { "text-ios-orange-dark" } else { "text-stone-400" },
                        {thread_theme.clone().unwrap_or_else(|| t(&lang, "thread-theme-none"))}
                    }
                }
            } else {
                span { class: "text-lg font-semibold tracking-[-0.01em] text-stone-900 flex-1", "{title}" }
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
            } else if is_thread {
                button {
                    class: "min-w-[44px] min-h-[44px] flex items-center justify-center text-stone-700 hover:text-stone-900 transition-colors duration-150",
                    onclick: move |_| {
                        app.show_folder_picker.set(false);
                        let cur = (app.show_thread_menu)();
                        app.show_thread_menu.set(!cur);
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
                        app.chat_scope.set(None);
                        app.previous_view.set(Some(View::NotesList));
                        app.view.set(View::Chat { conversation_id: None });
                    },
                    IconChatAi { size: 28 }
                }
            }
        }
    }
}
