use crate::application::i18n::t;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn ShortcutsSettings() -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();

    let rows: Vec<(Vec<&'static str>, String)> = vec![
        (vec!["⌘", "N"], t(&lang, "shortcut-new-note")),
        (vec!["⇧⌘Enter", "⇧⌃Enter"], t(&lang, "shortcut-new-chat")),
        (vec!["⌘", "F"], t(&lang, "shortcut-search")),
        (vec!["⌘", ","], t(&lang, "shortcut-settings")),
        (vec!["⌘⌘", "⌘1", "⌘2"], t(&lang, "shortcut-menus")),
        (vec!["⌃", "⌃"], t(&lang, "shortcut-theme-picker")),
        (vec!["⌘", "←"], t(&lang, "shortcut-back")),
        (vec!["⌘", "→"], t(&lang, "shortcut-forward")),
        (vec!["Esc"], t(&lang, "shortcut-escape")),
        (vec!["Esc", "Esc"], t(&lang, "shortcut-escape-double")),
        (vec!["Enter"], t(&lang, "shortcut-send")),
        (vec!["Shift", "Enter"], t(&lang, "shortcut-newline")),
    ];

    rsx! {
        div { class: "space-y-4 pb-20",
            div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                for row in rows.into_iter() {
                    {
                        let (keys, label) = row;
                        rsx! {
                            div { class: "flex items-center justify-between px-4 py-3 min-h-[44px] gap-3",
                                span { class: "text-sm text-stone-800", "{label}" }
                                div { class: "flex items-center gap-1 shrink-0",
                                    for k in keys.into_iter() {
                                        kbd { class: "px-2 py-0.5 rounded-lg bg-stone-100 border border-stone-200 text-xs font-medium text-stone-600",
                                            "{k}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "text-xs text-stone-400 px-1", {t(&lang, "shortcuts-hint")} }
        }
    }
}
