use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

// App Store 1.2 (user-generated content): terms + a way to reach us,
// one tap from Settings.
const TERMS_URL: &str = "https://mirkobozzetto.github.io/flowflow-privacy/";
const CONTACT_URL: &str = "mailto:mirko.prodev@gmail.com";

#[component]
pub fn PrivacySettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-6 pb-20",
            div {
                h2 { class: "text-lg font-semibold text-stone-900 mb-3",
                    {t(&lang, "settings-ai-services-title")}
                }
                p { class: "text-xs text-stone-500 mb-3 leading-relaxed",
                    {t(&lang, "settings-ai-services-description")}
                }
                button {
                    class: "w-full py-2.5 rounded-xl text-sm font-medium border border-stone-300 text-stone-500",
                    onclick: move |_| {
                        let _ = db().set_setting("ai_consent", "revoked");
                        app.ai_consent.set(None);
                    },
                    {t(&lang, "settings-revoke-consent")}
                }
            }
            div { class: "flex items-center justify-center gap-4",
                button {
                    class: "text-xs text-stone-500 underline underline-offset-2 active:text-stone-700",
                    onclick: move |_| {
                        crate::infrastructure::platform::open_url(TERMS_URL)
                    },
                    {t(&lang, "settings-legal-terms")}
                }
                button {
                    class: "text-xs text-stone-500 underline underline-offset-2 active:text-stone-700",
                    onclick: move |_| {
                        crate::infrastructure::platform::open_url(CONTACT_URL)
                    },
                    {t(&lang, "settings-legal-contact")}
                }
            }
            p { class: "text-xs text-stone-400 text-center",
                {t(&lang, "settings-keys-stored-locally")}
            }
        }
    }
}
