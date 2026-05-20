use crate::db::Database;
use crate::services::audio;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn SettingsView() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();

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
    let mut confirm_cleanup = use_signal(|| false);
    let mut cleanup_status: Signal<Option<String>> = use_signal(|| None);
    let audio_count = db().all_audio_paths().map(|p| p.len()).unwrap_or(0);

    rsx! {
        div { class: "space-y-6 pb-20",
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

            h2 { class: "text-lg font-semibold text-stone-900 pt-2", "Recherche" }
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

            div { class: "border-t border-stone-200 pt-4",
                h2 { class: "text-lg font-semibold text-stone-900 mb-3", "Stockage" }
                if let Some(ref status) = cleanup_status() {
                    p { class: "text-xs text-ios-green text-center mb-2", "{status}" }
                }
                if audio_count > 0 {
                    {
                        let label = format!("Nettoyer les fichiers audio ({audio_count})");
                        rsx! {
                            button {
                                class: if confirm_cleanup() {
                                    "w-full py-2.5 rounded-xl text-sm font-medium bg-ios-red text-white"
                                } else {
                                    "w-full py-2.5 rounded-xl text-sm font-medium border border-stone-300 text-stone-600"
                                },
                                onclick: move |_| {
                                    if confirm_cleanup() {
                                        let dir = audio::output_dir();
                                        match db().delete_all_audios(&dir) {
                                            Ok(count) => {
                                                cleanup_status.set(Some(
                                                    format!("{count} fichiers audio supprimés"),
                                                ));
                                                app.notes_version
                                                    .set((app.notes_version)() + 1);
                                            }
                                            Err(e) => {
                                                cleanup_status
                                                    .set(Some(format!("Erreur: {e}")));
                                            }
                                        }
                                        confirm_cleanup.set(false);
                                    } else {
                                        confirm_cleanup.set(true);
                                        spawn(async move {
                                            tokio::time::sleep(
                                                tokio::time::Duration::from_secs(3),
                                            )
                                            .await;
                                            confirm_cleanup.set(false);
                                        });
                                    }
                                },
                                if confirm_cleanup() {
                                    "Confirmer ? Les notes restent intactes."
                                } else {
                                    "{label}"
                                }
                            }
                        }
                    }
                } else {
                    p { class: "text-xs text-stone-400 text-center", "Aucun fichier audio" }
                }
            }

            p { class: "text-xs text-stone-400 text-center",
                "Les clés sont stockées localement sur cet appareil."
            }

            div { class: "border-t border-stone-200 pt-4",
                h2 { class: "text-lg font-semibold text-stone-900 mb-3",
                    "Services IA"
                }
                p { class: "text-xs text-stone-500 mb-3 leading-relaxed",
                    "Soniox, OpenAI et Anthropic sont utilisés pour la transcription, "
                    "la recherche sémantique et le chat."
                }
                button {
                    class: "w-full py-2.5 rounded-xl text-sm font-medium border border-stone-300 text-stone-500",
                    onclick: move |_| {
                        let _ = db().set_setting("ai_consent", "revoked");
                        app.ai_consent.set(None);
                    },
                    "Révoquer le consentement"
                }
            }
        }
    }
}
