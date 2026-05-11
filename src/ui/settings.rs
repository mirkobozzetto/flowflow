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
    let mut saved = use_signal(|| false);

    rsx! {
        div { class: "space-y-6",
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
                    let sk = soniox_key().trim().to_string();
                    if !ok.is_empty() {
                        let _ = db().set_setting("openai_api_key", &ok);
                    }
                    if !sk.is_empty() {
                        let _ = db().set_setting("soniox_api_key", &sk);
                    }
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
