use crate::db::Database;
use crate::services::backup;
use crate::services::i18n::{t, t_args};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn BackupSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut backup_status: Signal<Option<String>> = use_signal(|| None);
    let mut backup_busy = use_signal(|| false);
    let mut pending_import: Signal<Option<backup::ValidatedImport>> =
        use_signal(|| None);
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-6 pb-20",
            div {
                h2 { class: "text-lg font-semibold text-stone-900 mb-3",
                    {t(&lang, "settings-backup-title")}
                }
                p { class: "text-xs text-stone-500 mb-3 leading-relaxed",
                    {t(&lang, "settings-backup-description")}
                }
                if let Some(ref status) = backup_status() {
                    p { class: "text-xs text-stone-500 text-center mb-2", "{status}" }
                }
                if let Some(ref import) = pending_import() {
                    div { class: "rounded-xl border border-stone-300 bg-warm-white p-3 mb-3 space-y-2",
                        p { class: "text-sm font-medium text-stone-800",
                            {t_args(&lang, "settings-import-summary", &[
                                ("notes", &import.manifest.counts.notes.to_string()),
                                ("audio", &import.manifest.counts.audio_files.to_string()),
                                ("conversations", &import.manifest.counts.conversations.to_string()),
                            ])}
                        }
                        if import.same_lineage {
                            p { class: "text-xs text-stone-600",
                                {t(&lang, "settings-import-warning-same")}
                            }
                        } else {
                            p { class: "text-xs font-semibold text-ios-red",
                                {t(&lang, "settings-import-warning-other")}
                            }
                        }
                        div { class: "flex gap-2",
                            button {
                                class: "flex-1 py-2 rounded-xl text-sm font-medium bg-ios-red text-white",
                                onclick: {
                                    let lang = lang.clone();
                                    move |_| {
                                        let Some(import) = pending_import() else { return; };
                                        match backup::stage_import(&import) {
                                            Ok(()) => {
                                                pending_import.set(None);
                                            }
                                            Err(e) => {
                                                backup_status.set(Some(t_args(
                                                    &lang,
                                                    "settings-import-error",
                                                    &[("error", e.as_str())],
                                                )));
                                                pending_import.set(None);
                                            }
                                        }
                                    }
                                },
                                {t(&lang, "settings-import-confirm")}
                            }
                            button {
                                class: "flex-1 py-2 rounded-xl text-sm font-medium border border-stone-300 text-stone-600",
                                onclick: move |_| {
                                    let _ = std::fs::remove_dir_all(
                                        backup::import_staging_dir(),
                                    );
                                    pending_import.set(None);
                                },
                                {t(&lang, "settings-import-cancel")}
                            }
                        }
                    }
                }
                div { class: "space-y-2",
                    button {
                        class: crate::ui::kit::BTN_PRIMARY,
                        disabled: backup_busy(),
                        onclick: {
                            let lang = lang.clone();
                            move |_| {
                                if backup_busy() || backup::restore_lock_active() {
                                    return;
                                }
                                backup_busy.set(true);
                                backup_status.set(Some(t(&lang, "settings-exporting")));
                                let lang = lang.clone();
                                spawn(async move {
                                    let result: Result<(), String> = async {
                                        let store =
                                            crate::services::vectordb::VectorStore::open()
                                                .await?;
                                        let archive =
                                            backup::export_archive(&db(), &store).await?;
                                        #[cfg(target_os = "ios")]
                                        {
                                            crate::platform::ios::share_file(&archive)?;
                                            backup_status.set(None);
                                        }
                                        #[cfg(not(target_os = "ios"))]
                                        {
                                            match backup::save_archive_dialog(&archive)? {
                                                Some(_) => backup_status.set(Some(
                                                    t(&lang, "settings-export-saved"),
                                                )),
                                                None => backup_status.set(Some(
                                                    t(&lang, "settings-export-cancelled"),
                                                )),
                                            }
                                        }
                                        Ok(())
                                    }
                                    .await;
                                    if let Err(e) = result {
                                        backup_status.set(Some(t_args(
                                            &lang,
                                            "settings-export-error",
                                            &[("error", e.as_str())],
                                        )));
                                    }
                                    backup_busy.set(false);
                                });
                            }
                        },
                        {t(&lang, "settings-export")}
                    }
                    button {
                        class: "w-full py-2.5 rounded-xl text-sm font-medium border border-stone-300 text-stone-700 disabled:opacity-50",
                        disabled: backup_busy(),
                        onclick: {
                            let lang = lang.clone();
                            move |_| {
                                if backup_busy() || backup::restore_lock_active() {
                                    return;
                                }
                                backup_busy.set(true);
                                backup_status
                                    .set(Some(t(&lang, "settings-import-validating")));
                                let lang = lang.clone();
                                spawn(async move {
                                    let picked: Option<std::path::PathBuf>;
                                    #[cfg(target_os = "ios")]
                                    {
                                        picked = crate::platform::ios::open_file_picker(
                                            &["zip"],
                                        )
                                        .await
                                        .and_then(|v| v.into_iter().next());
                                    }
                                    #[cfg(not(target_os = "ios"))]
                                    {
                                        picked = rfd::FileDialog::new()
                                            .add_filter("FlowFlow backup", &["zip"])
                                            .pick_file();
                                    }
                                    let Some(path) = picked else {
                                        backup_status.set(None);
                                        backup_busy.set(false);
                                        return;
                                    };
                                    let live_id = db().get_setting("sync_device_id");
                                    match backup::validate_archive(
                                        &path,
                                        live_id.as_deref(),
                                    ) {
                                        Ok(v) => {
                                            backup_status.set(None);
                                            pending_import.set(Some(v));
                                        }
                                        Err(e) => backup_status.set(Some(t_args(
                                            &lang,
                                            "settings-import-error",
                                            &[("error", e.as_str())],
                                        ))),
                                    }
                                    backup_busy.set(false);
                                });
                            }
                        },
                        {t(&lang, "settings-import")}
                    }
                }
            }
        }
    }
}
