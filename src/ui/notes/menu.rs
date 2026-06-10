use crate::services::embed::delete_note_embeddings;
use crate::services::i18n::t;
#[cfg(target_os = "ios")]
use crate::services::i18n::t_args;
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

pub struct ImportedFile {
    pub filename: String,
    pub content: String,
}

pub async fn import_file_content(
    lang: &str,
) -> Result<Option<ImportedFile>, String> {
    #[cfg(target_os = "ios")]
    {
        use crate::platform::ios::{
            open_file_picker, read_file_as_text, read_pdf_text,
        };
        const MAX_FILE_SIZE: u64 = 20 * 1024 * 1024;
        let paths = match open_file_picker(&["txt", "md", "csv", "pdf", "docx"])
            .await
        {
            Some(p) => p,
            None => return Ok(None),
        };
        let path = match paths.first() {
            Some(p) => p,
            None => return Ok(None),
        };
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if file_size > MAX_FILE_SIZE {
            let size = (file_size / (1024 * 1024)).to_string();
            return Err(t_args(
                lang,
                "note-file-too-large",
                &[("size", &size)],
            ));
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document")
            .to_string();
        let is_pdf = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
        let content = if is_pdf {
            read_pdf_text(path)
        } else {
            let timeout_secs = 60 + (file_size / (1024 * 1024)) * 30;
            let path_owned = path.clone();
            let handle = tokio::task::spawn_blocking(move || {
                read_file_as_text(&path_owned)
            });
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(timeout_secs),
                handle,
            )
            .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(_)) => Err(t(lang, "note-extract-interrupted")),
                Err(_) => {
                    let mins = (timeout_secs / 60).to_string();
                    Err(t_args(
                        lang,
                        "note-file-too-complex",
                        &[("mins", &mins)],
                    ))
                }
            }
        };
        match content {
            Ok(text) => Ok(Some(ImportedFile {
                filename,
                content: text,
            })),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = lang;
        Ok(None)
    }
}

pub async fn import_audio_file() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "ios")]
    {
        use crate::platform::ios::open_audio_picker;
        let paths = open_audio_picker().await?;
        paths.into_iter().next()
    }
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

#[component]
pub fn NoteMenu(
    note_id: String,
    import_requested: Signal<bool>,
    deleted: Signal<bool>,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<crate::db::Database>> = use_context();
    let engine: Signal<Arc<crate::services::sync::engine::SyncEngine>> =
        use_context();
    let mut deleted = deleted;
    let mut import_requested = import_requested;
    let lang = (app.current_lang)();
    let import_label = t(&lang, "note-menu-import");
    let import_audio_label = t(&lang, "note-menu-import-audio");
    let delete_label = t(&lang, "note-menu-delete");

    rsx! {
        div {
            class: "fixed inset-0 z-40",
            onclick: move |_| app.show_note_menu.set(false),
        }
        div {
            class: "absolute right-4 top-1 z-50 bg-warm-white rounded-xl shadow-lg border border-stone-100 py-1 min-w-[200px]",
            button {
                class: "w-full flex items-center gap-3 px-4 py-3 text-sm text-stone-700 active:bg-stone-50",
                onclick: move |_| {
                    import_requested.set(true);
                    app.show_note_menu.set(false);
                },
                IconFileArrowUp { size: 18 }
                "{import_label}"
            }
            button {
                class: "w-full flex items-center gap-3 px-4 py-3 text-sm text-stone-700 active:bg-stone-50",
                onclick: move |_| {
                    app.audio_import_requested.set(true);
                    app.show_note_menu.set(false);
                },
                IconMic { size: 18 }
                "{import_audio_label}"
            }
            button {
                class: "w-full flex items-center gap-3 px-4 py-3 text-sm text-ios-red active:bg-stone-50",
                onclick: {
                    let note_id = note_id.clone();
                    move |_| {
                        app.show_note_menu.set(false);
                        deleted.set(true);
                        for a in db().list_audios(&note_id).unwrap_or_default() {
                            let _ = std::fs::remove_file(crate::services::audio::resolve_audio_path(&a.file_path));
                        }
                        let _ = db().delete_note(&note_id);
                        delete_note_embeddings(note_id.clone());
                        engine.peek().schedule_debounced();
                        app.current_note_id.set(None);
                    }
                },
                IconTrash { size: 18 }
                "{delete_label}"
            }
        }
    }
}
