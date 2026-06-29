use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::hooks::swipe::use_sheet_dismiss;
use crate::ui::icons::*;
use crate::ui::state::{SettingsSection, View};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

pub(crate) const ROW: &str = "w-full flex items-center gap-3 px-3 min-h-[48px] rounded-lg text-left hover:bg-stone-100 active:bg-stone-100 transition-colors duration-150";
const ROW_DISABLED: &str = "w-full flex items-center gap-3 px-3 min-h-[48px] rounded-lg text-left opacity-50";
pub(crate) const LEAD_ICON: &str = "w-[22px] h-[22px] shrink-0 flex items-center justify-center text-stone-500";
const LEAD_TILE: &str = "w-7 h-7 shrink-0 flex items-center justify-center rounded-lg bg-stone-100 overflow-hidden";
pub(crate) const ROW_TITLE: &str = "text-sm font-medium text-stone-800";
const ROW_SUB: &str = "text-xs text-stone-400";
pub(crate) const SECTION: &str =
    "px-3 py-2 text-xs font-medium text-stone-400 uppercase tracking-wide";
pub(crate) const SEP: &str = "h-px bg-stone-200 mx-3 my-2";
const PILL: &str = "shrink-0 h-7 px-3 flex items-center justify-center rounded-lg bg-stone-100 text-stone-900 text-xs font-medium";

#[component]
pub fn ToolsMenu() -> Element {
    let mut app: AppState = use_context();

    // Drag the grabber strip down to dismiss the sheet (finger-follow, then a
    // slide-out before Rust unmounts it). No-op on desktop: there is no #tools-sheet.
    use_sheet_dismiss("tools-sheet", "tools-backdrop", 48.0, 0.3, move || {
        app.show_tools_menu.set(false)
    });

    let close = move |_| app.show_tools_menu.set(false);

    if cfg!(target_os = "ios") {
        rsx! {
            div {
                id: "tools-backdrop",
                class: "fixed inset-0 z-30 bg-black/10 backdrop-fade",
                onclick: close,
            }
            div {
                id: "tools-sheet",
                class: "fixed bottom-0 left-0 right-0 z-40 bg-warm-white rounded-t-2xl px-2 pt-2 sheet-pop",
                style: "padding-bottom: calc(1.5rem + env(safe-area-inset-bottom));",
                div { class: "w-9 h-1 rounded-full bg-stone-300 mx-auto mt-1 mb-3" }
                ToolsMenuBody {}
            }
        }
    } else {
        rsx! {
            div { class: "fixed inset-0 z-30", onclick: close }
            div { class: "absolute bottom-full left-0 mb-2 w-80 z-40 bg-warm-white border border-stone-200 rounded-xl shadow-lg p-2 popover-pop",
                ToolsMenuBody {}
            }
        }
    }
}

#[component]
fn ToolsMenuBody() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let web_on = (app.chat_web)();
    let has_exa = db()
        .get_setting("exa_api_key")
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);

    let mut open_connections = move || {
        app.show_tools_menu.set(false);
        app.view
            .set(View::SettingsSection(SettingsSection::Connections));
    };

    rsx! {
        div { class: SECTION, {t(&lang, "chat-tools-section-tools")} }

        button {
            class: ROW,
            onclick: move |_| app.chat_web.set(!(app.chat_web)()),
            span { class: LEAD_ICON, IconGlobeSimple { size: 22 } }
            span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                span { class: ROW_TITLE, {t(&lang, "chat-tools-web")} }
                span { class: ROW_SUB,
                    if has_exa {
                        {t(&lang, "chat-tools-web-desc")}
                    } else {
                        {t(&lang, "chat-tools-web-needs-key")}
                    }
                }
            }
            Switch { on: web_on }
        }

        div {
            class: ROW_DISABLED,
            span { class: LEAD_ICON, IconMagnifyingGlass { size: 22 } }
            span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                span { class: ROW_TITLE, {t(&lang, "chat-tools-deep")} }
                span { class: ROW_SUB, {t(&lang, "chat-tools-deep-desc")} }
            }
            Switch { on: false }
        }

        div { class: SEP }
        div { class: SECTION, {t(&lang, "chat-tools-section-connectors")} }

        button {
            class: ROW,
            onclick: move |_| open_connections(),
            span { class: LEAD_TILE, IconGoogleSheets { size: 18 } }
            span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                span { class: ROW_TITLE, "Google Sheets" }
                span { class: ROW_SUB, {t(&lang, "chat-tools-connected")} }
            }
            span { class: "shrink-0 flex items-center gap-1 text-ios-orange text-xs font-medium",
                IconCheck { size: 16 }
            }
        }

        button {
            class: ROW,
            onclick: move |_| open_connections(),
            span { class: LEAD_TILE, IconGmail { size: 18 } }
            span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                span { class: ROW_TITLE, "Gmail" }
                span { class: ROW_SUB, {t(&lang, "chat-tools-gmail-desc")} }
            }
            span { class: PILL, {t(&lang, "chat-tools-connect")} }
        }

        button {
            class: ROW,
            onclick: move |_| open_connections(),
            span { class: LEAD_TILE, IconGoogleCalendar { size: 20 } }
            span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                span { class: ROW_TITLE, {t(&lang, "chat-conn-calendar")} }
                span { class: ROW_SUB, {t(&lang, "chat-tools-calendar-desc")} }
            }
            span { class: PILL, {t(&lang, "chat-tools-connect")} }
        }

        div {
            class: ROW_DISABLED,
            span { class: LEAD_TILE, IconNotion { size: 18 } }
            span { class: "flex-1 flex flex-col gap-0.5 min-w-0",
                span { class: ROW_TITLE, "Notion" }
                span { class: ROW_SUB, {t(&lang, "chat-tools-soon")} }
            }
        }

        div { class: "flex items-center gap-1.5 px-3 pt-3 text-xs text-stone-400",
            span { class: "px-1.5 py-0.5 rounded bg-stone-200 border border-stone-300 text-stone-600 font-mono text-[11px]",
                "@"
            }
            span { {t(&lang, "chat-tools-mention-hint")} }
        }
    }
}

#[component]
fn Switch(on: bool) -> Element {
    let track = if on {
        "relative w-11 h-6 shrink-0 rounded-full bg-ios-orange transition-colors duration-200"
    } else {
        "relative w-11 h-6 shrink-0 rounded-full bg-stone-300 transition-colors duration-200"
    };
    let knob = if on {
        "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-5"
    } else {
        "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-0"
    };
    rsx! {
        span { class: track,
            span { class: knob }
        }
    }
}
