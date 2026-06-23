use crate::application::i18n::t;
use crate::infrastructure::audio;
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn StorageSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let mut app: AppState = use_context();
    let mut confirm_cleanup = use_signal(|| false);
    let mut cleanup_status: Signal<Option<String>> = use_signal(|| None);
    let audio_count = db().all_audio_paths().map(|p| p.len()).unwrap_or(0);
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-6 pb-20",
            div {
                h2 { class: "text-lg font-semibold text-stone-900 mb-3", {t(&lang, "settings-storage-title")} }
                if let Some(ref status) = cleanup_status() {
                    p { class: "text-xs text-ios-green text-center mb-2", "{status}" }
                }
                if audio_count > 0 {
                    {
                        let label = format!("{} ({})", t(&lang, "settings-cleanup-audio"), audio_count);
                        rsx! {
                            button {
                                class: if confirm_cleanup() {
                                    "w-full py-2.5 rounded-xl text-sm font-medium bg-ios-red text-white"
                                } else {
                                    "w-full py-2.5 rounded-xl text-sm font-medium border border-stone-300 text-stone-600"
                                },
                                onclick: {
                                    let lang = lang.clone();
                                    move |_| {
                                    if confirm_cleanup() {
                                        let dir = audio::output_dir();
                                        match db().delete_all_audios(&dir) {
                                            Ok(count) => {
                                                cleanup_status.set(Some(
                                                    format!("{} {}", count, t(&lang, "settings-cleanup-done")),
                                                ));
                                                app.notes_version
                                                    .set((app.notes_version)() + 1);
                                            }
                                            Err(e) => {
                                                cleanup_status
                                                    .set(Some(format!("{}: {}", t(&lang, "settings-cleanup-error"), e)));
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
                                }
                                },
                                if confirm_cleanup() {
                                    {t(&lang, "settings-cleanup-confirm")}
                                } else {
                                    "{label}"
                                }
                            }
                        }
                    }
                } else {
                    p { class: "text-xs text-stone-400 text-center", {t(&lang, "settings-no-audio")} }
                }
            }
        }
    }
}
