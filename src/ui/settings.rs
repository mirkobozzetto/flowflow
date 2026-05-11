use crate::db::Database;
use crate::services::llm::Provider;
use dioxus::prelude::*;
use std::str::FromStr;
use std::sync::Arc;

#[component]
pub fn SettingsView() -> Element {
    let db: Signal<Arc<Database>> = use_context();

    let mut openai_key =
        use_signal(|| db().get_setting("openai_api_key").unwrap_or_default());
    let mut anthropic_key = use_signal(|| {
        db().get_setting("anthropic_api_key").unwrap_or_default()
    });
    let mut soniox_key =
        use_signal(|| db().get_setting("soniox_api_key").unwrap_or_default());
    let mut provider = use_signal(|| {
        db().get_setting("llm_provider")
            .and_then(|v| Provider::from_str(&v).ok())
            .unwrap_or_default()
    });
    let mut saved = use_signal(|| false);

    rsx! {
        div { class: "space-y-6",
            h2 { class: "text-lg font-semibold text-gray-900", "Fournisseur LLM" }
            div { class: "grid grid-cols-2 gap-2",
                button {
                    class: if provider() == Provider::OpenAi {
                        "py-2.5 rounded-xl text-sm font-medium bg-ios-blue text-white"
                    } else {
                        "py-2.5 rounded-xl text-sm font-medium bg-gray-100 text-gray-700"
                    },
                    onclick: move |_| {
                        provider.set(Provider::OpenAi);
                        saved.set(false);
                    },
                    "OpenAI"
                }
                button {
                    class: if provider() == Provider::Anthropic {
                        "py-2.5 rounded-xl text-sm font-medium bg-ios-blue text-white"
                    } else {
                        "py-2.5 rounded-xl text-sm font-medium bg-gray-100 text-gray-700"
                    },
                    onclick: move |_| {
                        provider.set(Provider::Anthropic);
                        saved.set(false);
                    },
                    "Anthropic"
                }
            }

            h2 { class: "text-lg font-semibold text-gray-900", "Clés API" }
            div { class: "space-y-4",
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1", "OpenAI API Key" }
                    input {
                        class: "w-full border border-gray-200 rounded-xl px-3 py-2.5 text-sm outline-none text-gray-900 bg-white",
                        r#type: "password",
                        placeholder: "sk-...",
                        value: "{openai_key}",
                        oninput: move |evt| {
                            openai_key.set(evt.value());
                            saved.set(false);
                        },
                    }
                    p { class: "text-xs text-gray-400 mt-1",
                        "Requise pour les embeddings (toujours utilisée)."
                    }
                }
                if provider() == Provider::Anthropic {
                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Anthropic API Key" }
                        input {
                            class: "w-full border border-gray-200 rounded-xl px-3 py-2.5 text-sm outline-none text-gray-900 bg-white",
                            r#type: "password",
                            placeholder: "sk-ant-...",
                            value: "{anthropic_key}",
                            oninput: move |evt| {
                                anthropic_key.set(evt.value());
                                saved.set(false);
                            },
                        }
                    }
                }
                div {
                    label { class: "block text-sm font-medium text-gray-700 mb-1", "Soniox API Key" }
                    input {
                        class: "w-full border border-gray-200 rounded-xl px-3 py-2.5 text-sm outline-none text-gray-900 bg-white",
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
            button {
                class: if saved() {
                    "w-full py-2.5 rounded-xl text-sm font-medium bg-ios-green text-white"
                } else {
                    "w-full py-2.5 rounded-xl text-sm font-medium bg-ios-blue text-white"
                },
                onclick: move |_| {
                    let ok = openai_key().trim().to_string();
                    let ak = anthropic_key().trim().to_string();
                    let sk = soniox_key().trim().to_string();
                    if !ok.is_empty() {
                        let _ = db().set_setting("openai_api_key", &ok);
                    }
                    if !ak.is_empty() {
                        let _ = db().set_setting("anthropic_api_key", &ak);
                    }
                    if !sk.is_empty() {
                        let _ = db().set_setting("soniox_api_key", &sk);
                    }
                    let _ = db().set_setting("llm_provider", provider().as_str());
                    saved.set(true);
                },
                if saved() { "Enregistré ✓" } else { "Enregistrer" }
            }
            p { class: "text-xs text-gray-400 text-center",
                "Les clés sont stockées localement sur cet appareil."
            }
        }
    }
}
