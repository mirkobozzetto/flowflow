use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn IntelligenceSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut openai_key =
        use_signal(|| db().get_setting("openai_api_key").unwrap_or_default());
    let mut exa_key =
        use_signal(|| db().get_setting("exa_api_key").unwrap_or_default());
    let mut web_search = use_signal(|| {
        db().get_setting("web_search_enabled").as_deref() == Some("true")
    });
    let mut max_sources = use_signal(|| {
        db().get_setting("rag_max_sources")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(8)
    });
    let mut saved = use_signal(|| false);
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-6 pb-20",
            h2 { class: "text-lg font-semibold text-stone-900", {t(&lang, "settings-api-keys-title")} }
            div {
                label { class: "block text-sm font-medium text-stone-700 mb-1", {t(&lang, "settings-openai-label")} }
                input {
                    class: crate::ui::kit::INPUT,
                    r#type: "password",
                    placeholder: "sk-...",
                    value: "{openai_key}",
                    oninput: move |evt| {
                        openai_key.set(evt.value());
                        saved.set(false);
                    },
                }
            }

            h2 { class: "text-lg font-semibold text-stone-900 pt-2", {t(&lang, "settings-search-title")} }
            div {
                label { class: "block text-sm font-medium text-stone-700 mb-1", {t(&lang, "settings-exa-label")} }
                input {
                    class: crate::ui::kit::INPUT,
                    r#type: "password",
                    placeholder: t(&lang, "settings-exa-placeholder"),
                    value: "{exa_key}",
                    oninput: move |evt| {
                        exa_key.set(evt.value());
                        saved.set(false);
                    },
                }
            }
            div {
                div { class: "flex items-center justify-between",
                    label { class: "text-sm font-medium text-stone-700",
                        {t(&lang, "settings-web-search-label")}
                    }
                    button {
                        class: if web_search() {
                            "relative w-11 h-6 rounded-full bg-ios-green transition-colors duration-200"
                        } else {
                            "relative w-11 h-6 rounded-full bg-stone-300 transition-colors duration-200"
                        },
                        onclick: move |_| {
                            let next = !web_search();
                            web_search.set(next);
                            let _ = db().set_setting(
                                "web_search_enabled",
                                if next { "true" } else { "false" },
                            );
                        },
                        span {
                            class: if web_search() {
                                "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-5"
                            } else {
                                "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-0"
                            },
                        }
                    }
                }
                p { class: "text-xs text-stone-400 mt-1",
                    {t(&lang, "settings-web-search-hint")}
                }
            }
            div {
                div { class: "flex justify-between items-center mb-1",
                    label { class: "text-sm font-medium text-stone-700",
                        {t(&lang, "settings-max-sources")}
                    }
                    span { class: "text-sm font-semibold text-ios-orange",
                        "{max_sources}"
                    }
                }
                input {
                    class: "w-full accent-ios-orange",
                    r#type: "range",
                    min: "3",
                    max: "15",
                    value: "{max_sources}",
                    oninput: move |evt| {
                        if let Ok(v) = evt.value().parse::<i64>() {
                            max_sources.set(v);
                            saved.set(false);
                        }
                    },
                }
                div { class: "flex justify-between text-xs text-stone-400",
                    span { "3" }
                    span { "15" }
                }
            }

            button {
                class: if saved() { crate::ui::kit::BTN_SUCCESS } else { crate::ui::kit::BTN_PRIMARY },
                onclick: move |_| {
                    let ok = openai_key().trim().to_string();
                    if !ok.is_empty() {
                        let _ = db().set_setting("openai_api_key", &ok);
                    }
                    let ek = exa_key().trim().to_string();
                    if !ek.is_empty() {
                        let _ = db().set_setting("exa_api_key", &ek);
                    }
                    let _ = db().set_setting(
                        "rag_max_sources",
                        &max_sources().to_string(),
                    );
                    saved.set(true);
                },
                if saved() {
                    {t(&lang, "settings-saved")}
                } else {
                    {t(&lang, "settings-save")}
                }
            }

            p { class: "text-xs text-stone-400 text-center",
                {t(&lang, "settings-keys-stored-locally")}
            }
        }
    }
}
