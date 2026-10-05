use crate::application::i18n::t;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn ChatEmptyState(
    #[props(default)] hermes: bool,
    /// Hermes: its model and counts, in place of a hint.
    #[props(default)]
    facts: Option<String>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let (title, hint) = if hermes {
        (t(&lang, "hermes-title"), facts.unwrap_or_default())
    } else {
        (t(&lang, "chat-empty-title"), t(&lang, "chat-empty-hint"))
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
            p { class: "text-stone-900 font-semibold text-base mb-1", "{title}" }
            p { class: "text-stone-400 text-sm text-center min-h-5", "{hint}" }
        }
    }
}
