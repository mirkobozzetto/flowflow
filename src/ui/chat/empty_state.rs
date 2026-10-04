use crate::application::i18n::t;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn ChatEmptyState(#[props(default)] hermes: bool) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let (title, hint) = if hermes {
        ("hermes-empty-title", "hermes-empty-hint")
    } else {
        ("chat-empty-title", "chat-empty-hint")
    };
    rsx! {
        div {
            class: "flex flex-col items-center justify-center px-6 h-full",
            if hermes {
                img {
                    src: asset!("/assets/hermes-agent.png"),
                    width: "96",
                    height: "96",
                    class: "mb-6 rounded-full object-cover",
                    alt: "Hermes",
                }
            } else {
                img {
                    src: asset!("/assets/flowflow-icon-300.png"),
                    width: "96",
                    height: "96",
                    class: "mb-6 object-contain",
                }
            }
            p { class: "text-stone-900 font-semibold text-base mb-1",
                {t(&lang, title)}
            }
            p { class: "text-stone-400 text-sm text-center",
                {t(&lang, hint)}
            }
        }
    }
}
