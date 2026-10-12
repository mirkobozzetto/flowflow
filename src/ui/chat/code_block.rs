use crate::application::i18n::t;
use crate::ui::clipboard::copy_text;
use crate::ui::icons::{IconCheck, IconCopy};
use crate::ui::AppState;
use dioxus::prelude::*;

const COPIED_FOR_MS: u64 = 1500;

/// A code block of a reply: its language and a copy button above the code.
#[component]
pub fn CodeBlock(lang: String, code: String) -> Element {
    let app: AppState = use_context();
    let ui_lang = (app.current_lang)();
    let mut copied = use_signal(|| false);
    let to_copy = code.clone();
    rsx! {
        div { class: "code-frame",
            div { class: "code-head",
                span { class: "text-[11px] font-medium text-stone-500", "{lang}" }
                button {
                    class: "h-11 -my-1 px-3 inline-flex items-center gap-1 text-xs font-medium text-stone-600",
                    onclick: move |_| {
                        copy_text(&to_copy);
                        copied.set(true);
                        spawn(async move {
                            futures_timer::Delay::new(std::time::Duration::from_millis(COPIED_FOR_MS))
                                .await;
                            copied.set(false);
                        });
                    },
                    if copied() {
                        IconCheck { size: 14 }
                        {t(&ui_lang, "common-copied")}
                    } else {
                        IconCopy { size: 14 }
                        {t(&ui_lang, "common-copy")}
                    }
                }
            }
            pre {
                code { class: "language-{lang}", "{code}" }
            }
        }
    }
}
