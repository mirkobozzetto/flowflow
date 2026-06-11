use crate::db::Database;
use crate::services::i18n::t;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

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
            p { class: "text-xs text-stone-400 text-center",
                {t(&lang, "settings-keys-stored-locally")}
            }
        }
    }
}
