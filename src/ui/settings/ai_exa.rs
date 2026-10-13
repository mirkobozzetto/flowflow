use super::ai_card::{Brand, KeyBadge, ProviderCard, StatusMuted};
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::kit::{BTN_PRIMARY, INPUT, SECTION_LABEL};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

const KEY: &str = "exa_api_key";

#[component]
pub fn ExaSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut key = use_signal(|| db().get_setting(KEY).unwrap_or_default());
    let configured = use_signal(|| !key().trim().is_empty());

    rsx! {
        div { class: "space-y-6 pb-20",
            div {
                p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), {t(&lang, "settings-source-api-key")} }
                ProviderCard {
                    brand: Brand::Exa,
                    title: t(&lang, "settings-exa-label"),
                    status: rsx! { StatusMuted { text: t(&lang, "settings-web-search") } },
                    right: rsx! { KeyBadge { configured: configured() } },
                    input {
                        class: format!("{INPUT} mt-3"),
                        r#type: "password",
                        placeholder: t(&lang, "settings-exa-placeholder"),
                        value: "{key}",
                        oninput: move |evt| {
                            key.set(evt.value());
                        },
                    }
                }
            }
            p { class: "text-xs text-stone-400", {t(&lang, "settings-web-search-moved")} }
            button {
                class: BTN_PRIMARY,
                onclick: move |_| {
                    let value = key().trim().to_string();
                    if !value.is_empty() {
                        let _ = db().set_setting(KEY, &value);
                    }
                    super::ai_card::back_to_ai(app);
                },
                {t(&lang, "settings-save")}
            }
        }
    }
}
