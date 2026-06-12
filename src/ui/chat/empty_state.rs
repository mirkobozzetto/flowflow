use crate::services::i18n::t;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn ChatEmptyState() -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    rsx! {
        div {
            class: "flex flex-col items-center justify-center px-6 h-full",
            img {
                src: asset!("/assets/flowflow-icon-300.png"),
                width: "150",
                height: "150",
                class: "mb-6 rounded-2xl",
            }
            p { class: "text-stone-900 font-semibold text-base mb-1",
                {t(&lang, "chat-empty-title")}
            }
            p { class: "text-stone-400 text-sm text-center",
                {t(&lang, "chat-empty-hint")}
            }
        }
    }
}
