use crate::application::i18n::t;
use crate::infrastructure::persistence::settings_repo::LANGUAGE_KEY;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn GeneralSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-6 pb-20",
            div {
                div { class: "flex gap-2",
                    button {
                        class: if lang == "en" {
                            "flex-1 min-h-[44px] rounded-xl text-sm font-medium bg-ios-orange text-white"
                        } else {
                            "flex-1 min-h-[44px] rounded-xl text-sm font-medium bg-stone-200 text-stone-700"
                        },
                        onclick: move |_| {
                            let _ = db().set_setting(LANGUAGE_KEY, "en");
                            app.current_lang.set("en".to_string());
                        },
                        {t(&lang, "language-en")}
                    }
                    button {
                        class: if lang == "fr" {
                            "flex-1 min-h-[44px] rounded-xl text-sm font-medium bg-ios-orange text-white"
                        } else {
                            "flex-1 min-h-[44px] rounded-xl text-sm font-medium bg-stone-200 text-stone-700"
                        },
                        onclick: move |_| {
                            let _ = db().set_setting(LANGUAGE_KEY, "fr");
                            app.current_lang.set("fr".to_string());
                        },
                        {t(&lang, "language-fr")}
                    }
                }
            }
        }
    }
}
