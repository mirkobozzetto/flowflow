use crate::application::i18n::t;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn RestoreLockScreen() -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let hint_key = if cfg!(target_os = "ios") {
        "restore-lock-hint-ios"
    } else {
        "restore-lock-hint-desktop"
    };

    rsx! {
        div {
            class: "fixed inset-0 z-[80] bg-stone-100 flex flex-col items-center justify-center px-8",
            style: "padding-top: env(safe-area-inset-top); padding-bottom: env(safe-area-inset-bottom);",

            img {
                src: asset!("/assets/flowflow-icon-300.png"),
                class: "w-32 h-32 object-contain mb-8",
                alt: "FlowFlow",
            }
            h1 { class: "text-2xl font-bold text-stone-800 text-center mb-4",
                {t(&lang, "restore-lock-title")}
            }
            p { class: "text-base text-stone-600 text-center mb-3 max-w-[340px]",
                {t(&lang, "restore-lock-body")}
            }
            p { class: "text-sm text-stone-400 text-center max-w-[340px]",
                {t(&lang, hint_key)}
            }
        }
    }
}
