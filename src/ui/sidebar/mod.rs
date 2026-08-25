mod conversations;
mod folders;
mod space_section;

pub use conversations::*;
pub use folders::*;

use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::hooks::swipe::{use_swipe_drawer, DrawerSwipe};
use crate::ui::icons::*;
use crate::ui::{AppState, SidebarTab, View};
use dioxus::prelude::*;
use std::sync::Arc;

pub(crate) fn navigate_with_slide(mut app: AppState, target: View) {
    if (app.view)() == target {
        return;
    }
    if cfg!(target_os = "macos")
        || target != View::NotesList
        || (app.view)() == View::NotesList
    {
        app.previous_view.set(None);
        app.view.set(target);
        return;
    }
    app.sliding_out.set(true);
    // spawn_forever, NOT spawn: the caller is a sidebar row that can unmount within
    // the 150ms (deleted, list refresh); a scope-bound task would be cancelled
    // mid-delay, leaving sliding_out stuck true and the view never set.
    dioxus::core::spawn_forever(async move {
        futures_timer::Delay::new(std::time::Duration::from_millis(150)).await;
        app.sliding_out.set(false);
        app.previous_view.set(None);
        app.view.set(target);
    });
}

#[component]
pub fn SidebarOverlay() -> Element {
    let mut app: AppState = use_context();
    let _db: Signal<Arc<Database>> = use_context();
    let is_open = (app.sidebar_open)();
    let lang = (app.current_lang)();

    use_swipe_drawer(DrawerSwipe {
        open: app.sidebar_open,
        panel_id: "sb-panel",
        backdrop_id: "sb-backdrop",
        edge: "left",
        edge_px: 30.0,
        open_at: 0.15,
        close_at: 0.4,
    });

    rsx! {
        if !is_open {
            div {
                id: "sb-edge",
                class: "fixed left-0 top-0 h-full w-8 z-30 lg:hidden",
                style: "touch-action: none;",
            }
        }
        div {
            id: "sb-backdrop",
            class: "fixed inset-0 bg-black/35 z-40 transition-opacity duration-200 lg:hidden",
            class: if is_open { "opacity-100" } else { "opacity-0 pointer-events-none" },
            onclick: move |_| app.sidebar_open.set(false),
        }
        // Outside-click catcher for row context menus. It MUST live here - outside the
        // translated sb-panel (a `translate` ancestor contains `fixed` descendants) and
        // outside any row subtree - so deleting a row can never orphan it in the DOM.
        // Below the panel (z-40 < z-50): taps on other rows still land, and those
        // handlers close the menu themselves.
        if (app.row_menu)().is_some() {
            div {
                class: "fixed inset-0 z-40",
                onclick: move |_| app.row_menu.set(None),
            }
        }
        div {
            id: "sb-panel",
            "data-open": if is_open { "1" } else { "0" },
            class: "fixed left-0 top-0 w-[85vw] max-w-[340px] h-full bg-warm-white z-50 flex flex-col border-r border-stone-200 transition-transform duration-200 safe-pt lg:static lg:translate-x-0 lg:w-72 lg:shrink-0 lg:h-screen",
            class: if is_open { "translate-x-0" } else { "-translate-x-full" },
            onclick: move |evt| evt.stop_propagation(),

            div { class: "relative flex border-b border-stone-200 lg:h-[68px]",
                button {
                    class: if (app.sidebar_tab)() == SidebarTab::Notes {
                        "flex-1 flex items-center justify-center py-3 text-sm font-semibold text-ios-orange-dark transition-colors duration-200"
                    } else {
                        "flex-1 flex items-center justify-center py-3 text-sm font-medium text-stone-400 hover:text-stone-600 transition-colors duration-200"
                    },
                    onclick: move |_| app.sidebar_tab.set(SidebarTab::Notes),
                    div { class: "flex items-center justify-center gap-1.5",
                        IconNotePencil { size: 16 }
                        {t(&lang, "sidebar-tab-notes")}
                    }
                }
                button {
                    class: if (app.sidebar_tab)() == SidebarTab::Chats {
                        "flex-1 flex items-center justify-center py-3 text-sm font-semibold text-ios-orange-dark transition-colors duration-200"
                    } else {
                        "flex-1 flex items-center justify-center py-3 text-sm font-medium text-stone-400 hover:text-stone-600 transition-colors duration-200"
                    },
                    onclick: move |_| app.sidebar_tab.set(SidebarTab::Chats),
                    div { class: "flex items-center justify-center gap-1.5",
                        IconChats { size: 16 }
                        {t(&lang, "sidebar-tab-chats")}
                    }
                }
                div {
                    class: "absolute bottom-0 left-0 w-1/2 h-0.5 bg-ios-orange-dark transition-transform duration-200 ease-out",
                    class: if (app.sidebar_tab)() == SidebarTab::Chats { "translate-x-full" } else { "translate-x-0" },
                }
            }

            div { class: "flex-1 overflow-y-auto p-4",
                match (app.sidebar_tab)() {
                    SidebarTab::Notes => rsx! {
                        div { class: "py-2 pb-3",
                            button {
                                class: "flex items-center gap-2.5 w-full px-2 py-3 text-sm font-medium text-ios-orange-dark rounded-lg min-h-[44px] hover:bg-ios-orange-50 transition-colors duration-150",
                                onclick: move |_| {
                                    app.show_folder_picker.set(false);
                                    app.sidebar_open.set(false);
                                    if matches!((app.view)(), View::NoteDetail { .. }) {
                                        app.view.set(View::NotesList);
                                        spawn(async move {
                                            futures_timer::Delay::new(
                                                std::time::Duration::from_millis(30),
                                            )
                                            .await;
                                            app.view.set(View::NoteDetail { note_id: String::new() });
                                        });
                                    } else {
                                        app.view.set(View::NoteDetail { note_id: String::new() });
                                    }
                                },
                                IconPlus { size: 16 }
                                {t(&lang, "sidebar-new-note")}
                            }
                            button {
                                class: "flex items-center gap-2.5 w-full px-2 py-3 text-base text-stone-900 font-semibold rounded-lg min-h-[44px] hover:bg-stone-100 transition-colors duration-150",
                                onclick: move |_| {
                                    app.selected_folder_id.set(None);
                                    app.sidebar_open.set(false);
                                    navigate_with_slide(app, View::NotesList);
                                },
                                IconFiles { size: 20 }
                                {t(&lang, "sidebar-all-notes")}
                            }
                            crate::ui::shared::open_link::OpenShareLink {}
                        }
                        div { class: "h-px bg-stone-200 mb-3" }
                        FolderSection {}
                    },
                    SidebarTab::Chats => rsx! {
                        ConversationSection {}
                    },
                }
            }

            div { class: "border-t border-stone-200 p-4",
                button {
                    class: "flex items-center gap-2.5 w-full px-2 py-3 text-sm text-stone-500 rounded-lg min-h-[44px] lg:min-h-[48px] hover:bg-stone-100 transition-colors duration-150",
                    onclick: move |_| {
                        app.view.set(View::Settings);
                        app.sidebar_open.set(false);
                    },
                    IconGear { size: 18 }
                    {t(&lang, "sidebar-settings")}
                }
            }
        }
    }
}
