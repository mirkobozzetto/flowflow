use crate::application::i18n::t;
use crate::infrastructure::persistence::settings_repo::LANGUAGE_KEY;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ConsentScreen() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let mut save_failed = use_signal(|| false);
    let lang = (app.current_lang)();

    rsx! {
        main {
            class: "fixed inset-0 z-[60] overflow-y-auto bg-stone-100 text-stone-800",
            style: "padding-top: env(safe-area-inset-top); padding-bottom: env(safe-area-inset-bottom);",
            div { class: "min-h-full w-full max-w-3xl mx-auto px-6 py-8 sm:py-12 flex flex-col justify-center",
                header { class: "flex items-center justify-between gap-4",
                    span { class: "text-sm font-semibold tracking-tight", "FlowFlow" }
                    div { class: "flex items-center rounded-lg bg-stone-200/60 p-1",
                        for locale in ["fr", "en"] {
                            button {
                                class: if lang == locale {
                                    "min-h-[44px] px-3 rounded-md bg-warm-white text-sm font-medium text-stone-800"
                                } else {
                                    "min-h-[44px] px-3 rounded-md text-sm text-stone-600 hover:text-stone-900"
                                },
                                aria_pressed: lang == locale,
                                onclick: move |_| {
                                    if db().set_setting(LANGUAGE_KEY, locale).is_ok() {
                                        app.current_lang.set(locale.to_string());
                                    }
                                },
                                {t(&lang, if locale == "fr" { "language-fr" } else { "language-en" })}
                            }
                        }
                    }
                }
                section { class: "grid grid-cols-1 sm:grid-cols-[96px_minmax(0,1fr)] gap-7 sm:gap-8 py-8 sm:py-12",
                    img {
                        src: asset!("/assets/flowflow-icon-300.png"),
                        class: "w-20 h-20 sm:w-24 sm:h-24 object-contain",
                        alt: "",
                    }
                    div {
                    h1 { class: "text-3xl sm:text-4xl font-semibold tracking-tight leading-tight text-balance",
                        {t(&lang, "consent-title")}
                    }
                    p { class: "text-base text-stone-600 leading-relaxed mt-5",
                        {t(&lang, "consent-description")}
                    }
                }
                }
                footer { class: "space-y-4 pb-4 sm:ml-32",
                    p { class: "text-xs text-stone-600 leading-relaxed border-t border-stone-200 pt-5",
                        {t(&lang, "consent-disclaimer")}
                    }
                    if save_failed() {
                        p { class: "text-sm text-ios-red", role: "alert",
                            {t(&lang, "consent-save-error")}
                        }
                    }
                    div { class: "w-full sm:max-w-[220px]",
                    button {
                        class: crate::ui::kit::BTN_PRIMARY,
                        onclick: move |_| {
                            if db().set_setting("ai_consent", "true").is_ok() {
                                app.ai_consent.set(Some(true));
                            } else {
                                save_failed.set(true);
                            }
                        },
                        {t(&lang, "consent-cta")}
                    }
                    }
                }
            }
        }
    }
}
