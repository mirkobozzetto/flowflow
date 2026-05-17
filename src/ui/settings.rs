use crate::db::Database;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn SettingsView() -> Element {
    let db: Signal<Arc<Database>> = use_context();

    let mut openai_key =
        use_signal(|| db().get_setting("openai_api_key").unwrap_or_default());
    let mut soniox_key =
        use_signal(|| db().get_setting("soniox_api_key").unwrap_or_default());
    let mut max_sources = use_signal(|| {
        db().get_setting("rag_max_sources")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(8)
    });
    let mut saved = use_signal(|| false);

    rsx! {
        div { class: "space-y-6",
            h2 { class: "text-lg font-semibold text-stone-900", "Clés API" }
            div { class: "space-y-4",
                div {
                    label { class: "block text-sm font-medium text-stone-700 mb-1", "OpenAI API Key" }
                    input {
                        class: "w-full border border-stone-200 rounded-xl px-3 py-2.5 text-sm outline-none text-stone-900 bg-warm-white",
                        r#type: "password",
                        placeholder: "sk-...",
                        value: "{openai_key}",
                        oninput: move |evt| {
                            openai_key.set(evt.value());
                            saved.set(false);
                        },
                    }
                }
                div {
                    label { class: "block text-sm font-medium text-stone-700 mb-1", "Soniox API Key" }
                    input {
                        class: "w-full border border-stone-200 rounded-xl px-3 py-2.5 text-sm outline-none text-stone-900 bg-warm-white",
                        r#type: "password",
                        placeholder: "Clé Soniox",
                        value: "{soniox_key}",
                        oninput: move |evt| {
                            soniox_key.set(evt.value());
                            saved.set(false);
                        },
                    }
                }
            }

            h2 { class: "text-lg font-semibold text-stone-900 pt-2", "RAG" }
            div {
                div { class: "flex justify-between items-center mb-1",
                    label { class: "text-sm font-medium text-stone-700",
                        "Max sources"
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
                class: if saved() {
                    "w-full py-2.5 rounded-xl text-sm font-medium bg-ios-green text-white"
                } else {
                    "w-full py-2.5 rounded-xl text-sm font-medium bg-ios-orange text-white"
                },
                onclick: move |_| {
                    let ok = openai_key().trim().to_string();
                    let sk = soniox_key().trim().to_string();
                    if !ok.is_empty() {
                        let _ = db().set_setting("openai_api_key", &ok);
                    }
                    if !sk.is_empty() {
                        let _ = db().set_setting("soniox_api_key", &sk);
                    }
                    let _ = db().set_setting(
                        "rag_max_sources",
                        &max_sources().to_string(),
                    );
                    saved.set(true);
                },
                if saved() { "Enregistré ✓" } else { "Enregistrer" }
            }
            p { class: "text-xs text-stone-400 text-center",
                "Les clés sont stockées localement sur cet appareil."
            }
        }
    }
}
