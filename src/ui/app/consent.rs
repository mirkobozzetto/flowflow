use crate::application::i18n::t;
use crate::infrastructure::persistence::settings_repo::LANGUAGE_KEY;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

const SPARKLE: &str =
    "M12 0L14.59 9.41L24 12L14.59 14.59L12 24L9.41 14.59L0 12L9.41 9.41L12 0Z";

#[component]
pub fn ConsentScreen() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();

    rsx! {
        div {
            class: "fixed inset-0 z-[60] bg-stone-100 flex flex-col px-6",
            style: "padding-top: env(safe-area-inset-top); padding-bottom: env(safe-area-inset-bottom);",

            div { class: "flex-1 flex flex-col items-center justify-center",
                img {
                    src: asset!("/assets/flowflow-icon-300.png"),
                    class: "w-64 h-64 object-contain",
                    alt: "FlowFlow",
                }

                div { class: "relative bg-warm-white rounded-2xl border border-stone-200/60 p-5 mt-8 max-w-[320px] w-full",
                    svg {
                        class: "absolute -top-2 -right-2 text-ios-orange opacity-40",
                        width: "24",
                        height: "24",
                        view_box: "0 0 24 24",
                        fill: "currentColor",
                        path { d: SPARKLE }
                    }
                    svg {
                        class: "absolute top-6 -right-1 text-ios-orange opacity-20",
                        width: "14",
                        height: "14",
                        view_box: "0 0 24 24",
                        fill: "currentColor",
                        path { d: SPARKLE }
                    }

                    h2 { class: "font-semibold text-base text-stone-800 mb-2",
                        {t(&lang, "consent-title")}
                    }
                    p { class: "text-sm text-stone-500 leading-relaxed",
                        {t(&lang, "consent-description")}
                    }
                }
            }

            div { class: "w-full pb-8",
                div { class: "flex justify-center gap-2 mb-4",
                    button {
                        class: if lang == "en" {
                            "min-h-[44px] px-5 rounded-full text-sm font-medium bg-ios-orange text-white"
                        } else {
                            "min-h-[44px] px-5 rounded-full text-sm font-medium bg-stone-200 text-stone-700"
                        },
                        onclick: move |_| {
                            let _ = db().set_setting(LANGUAGE_KEY, "en");
                            app.current_lang.set("en".to_string());
                        },
                        {t(&lang, "language-en")}
                    }
                    button {
                        class: if lang == "fr" {
                            "min-h-[44px] px-5 rounded-full text-sm font-medium bg-ios-orange text-white"
                        } else {
                            "min-h-[44px] px-5 rounded-full text-sm font-medium bg-stone-200 text-stone-700"
                        },
                        onclick: move |_| {
                            let _ = db().set_setting(LANGUAGE_KEY, "fr");
                            app.current_lang.set("fr".to_string());
                        },
                        {t(&lang, "language-fr")}
                    }
                }
                button {
                    class: crate::ui::kit::BTN_HERO,
                    onclick: move |_| {
                        let _ = db().set_setting("ai_consent", "true");
                        app.ai_consent.set(Some(true));
                    },
                    {t(&lang, "consent-cta")}
                }
                p { class: "text-xs text-stone-400/50 text-center mt-3 px-4 leading-relaxed",
                    {t(&lang, "consent-disclaimer")}
                }
            }

            svg {
                class: "fixed bottom-28 left-8 text-ios-orange opacity-15 pointer-events-none",
                width: "18",
                height: "18",
                view_box: "0 0 24 24",
                fill: "currentColor",
                path { d: SPARKLE }
            }
        }
    }
}
