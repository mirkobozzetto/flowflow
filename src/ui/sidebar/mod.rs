mod conversations;
mod folders;
mod join_link;
mod space_section;

pub use conversations::*;
pub use folders::*;

use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::haptic;
use crate::ui::hooks::swipe::{use_swipe_drawer, DrawerSwipe};
use crate::ui::icons::*;
use crate::ui::{AppState, SidebarTab, View};
use dioxus::prelude::*;
use std::sync::Arc;

// A menu opened at the bottom of the drawer would sit below the fold: bring
// it into view once it is in the DOM.
fn use_menu_scroll(app: AppState) {
    use_effect(move || {
        if (app.row_menu)().is_some() {
            dioxus::document::eval(
                "requestAnimationFrame(function() {
                    var el = document.getElementById('row-menu');
                    if (el) el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
                });",
            );
        }
    });
}

/// iOS: the main view slides aside as a card over the menu (#177); elsewhere
/// the menu is a drawer sliding over the content.
pub(crate) const CARD_MODE: bool = cfg!(target_os = "ios");

// One light tick each time the menu commits open or closed (burger, drag
// release, row tap), and in card mode the keyboard steps aside on open.
fn use_open_feedback(open: Signal<bool>) {
    let last = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(false)));
    use_effect(move || {
        let now = open();
        // Re-applied on every change: WebKit may toggle the effect itself.
        #[cfg(target_os = "ios")]
        crate::infrastructure::platform::ios::hide_top_edge_effect();
        if last.replace(now) == now {
            return;
        }
        haptic("light");
        if now && CARD_MODE {
            dioxus::document::eval(
                "document.activeElement && document.activeElement.blur()",
            );
        }
    });
}

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
    let app_for_menu: AppState = use_context();
    use_menu_scroll(app_for_menu);
    let mut app: AppState = use_context();
    let _db: Signal<Arc<Database>> = use_context();
    let is_open = (app.sidebar_open)();
    let lang = (app.current_lang)();

    use_open_feedback(app.sidebar_open);
    use_swipe_drawer(DrawerSwipe {
        open: app.sidebar_open,
        panel_id: "sb-panel",
        backdrop_id: "sb-backdrop",
        card_id: if CARD_MODE { "main-card" } else { "" },
        edge: "left",
        edge_px: 30.0,
        // The card snaps past half its travel; the drawer opens early.
        open_at: if CARD_MODE { 0.5 } else { 0.15 },
        close_at: if CARD_MODE { 0.5 } else { 0.4 },
    });

    rsx! {
        if !is_open {
            div {
                id: "sb-edge",
                class: "fixed left-0 top-0 h-full w-8 z-30 lg:hidden",
                style: "touch-action: none;",
            }
        }
        // Card mode has no veil: the card's own cover closes the menu.
        if !CARD_MODE {
            div {
                id: "sb-backdrop",
                class: "fixed inset-0 bg-black/35 z-40 transition-opacity duration-200 lg:hidden",
                class: if is_open { "opacity-100" } else { "opacity-0 pointer-events-none" },
                onclick: move |_| app.sidebar_open.set(false),
            }
        }
        // Outside-click catcher for row context menus. It MUST live here - outside the
        // translated sb-panel (a `translate` ancestor contains `fixed` descendants) and
        // outside any row subtree - so deleting a row can never orphan it in the DOM.
        // Below the panel (z-40 < z-50): taps on other rows still land, and those
        // handlers close the menu themselves. Card mode: the panel's own onclick
        // and the card cover cover the whole screen, no catcher needed.
        if !CARD_MODE && (app.row_menu)().is_some() {
            div {
                class: "fixed inset-0 z-40",
                onclick: move |_| app.row_menu.set(None),
            }
        }
        div {
            id: "sb-panel",
            "data-open": if is_open { "1" } else { "0" },
            class: if CARD_MODE {
                "sb-under fixed left-0 top-0 h-full bg-warm-white flex flex-col safe-pt lg:static lg:w-72 lg:shrink-0 lg:h-screen lg:border-r lg:border-stone-200"
            } else {
                "fixed left-0 top-0 w-[85vw] max-w-[340px] h-full bg-warm-white z-50 flex flex-col border-r border-stone-200 transition-transform duration-200 safe-pt lg:static lg:translate-x-0 lg:w-72 lg:shrink-0 lg:h-screen"
            },
            class: if CARD_MODE { "" } else if is_open { "translate-x-0" } else { "-translate-x-full" },
            // A tap anywhere in the drawer that is not on a menu closes the
            // open menu; menus and their buttons stop the bubble themselves.
            onclick: move |evt| {
                evt.stop_propagation();
                app.row_menu.set(None);
            },

            button {
                class: "h-[68px] shrink-0 flex items-center gap-2.5 px-5 border-b border-stone-200 text-left hover:bg-stone-50 transition-colors duration-150",
                onclick: move |_| {
                    app.sidebar_tab.set(SidebarTab::Notes);
                    app.selected_folder_id.set(None);
                    app.sidebar_open.set(false);
                    navigate_with_slide(app, View::NotesList);
                },
                img {
                    src: asset!("/assets/flowflow-icon-300.png"),
                    // As big as the glass burger on mobile (#177).
                    class: "w-12 h-12 lg:w-6 lg:h-6 object-contain",
                    alt: "",
                }
                span { class: "text-[15px] font-semibold tracking-[-0.01em] text-stone-900", "FlowFlow" }
            }
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

            div { class: "flex-1 overflow-y-auto overflow-x-hidden p-4 pb-32",
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
                                class: if (app.selected_folder_id)().is_none() && matches!((app.view)(), View::NotesList) {
                                    "flex items-center gap-2.5 w-full px-2 py-3 text-base font-semibold text-ios-orange-dark bg-ios-orange-50 rounded-lg min-h-[44px]"
                                } else {
                                    "flex items-center gap-2.5 w-full px-2 py-3 text-base text-stone-900 font-semibold rounded-lg min-h-[44px] hover:bg-stone-100 transition-colors duration-150"
                                },
                                onclick: move |_| {
                                    app.selected_folder_id.set(None);
                                    app.sidebar_open.set(false);
                                    navigate_with_slide(app, View::NotesList);
                                },
                                IconFiles { size: 20 }
                                {t(&lang, "sidebar-all-notes")}
                            }
                        }
                        div { class: "border-t border-stone-200/70 mb-3" }
                        FolderSection {}
                    },
                    SidebarTab::Chats => rsx! {
                        ConversationSection {}
                    },
                }
            }

            div { class: "border-t border-stone-200 p-4",
                button {
                    class: "glass-disc glass-pill pressable relative inline-flex items-center gap-2 pl-3 pr-4 min-h-[44px] rounded-full text-base font-medium text-stone-800",
                    onclick: move |_| {
                        app.view.set(View::Settings);
                        app.sidebar_open.set(false);
                    },
                    IconGear { size: 22 }
                    span { {t(&lang, "sidebar-settings")} }
                }
            }
        }
    }
}
