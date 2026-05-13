use crate::services::embed::delete_note_embeddings;
use crate::ui::icons::*;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

pub struct ImportedFile {
    pub filename: String,
    pub content: String,
}

pub async fn import_file_content() -> Result<Option<ImportedFile>, String> {
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
            return Err(format!(
                "Fichier trop volumineux ({} MB, max 20 MB)",
                file_size / (1024 * 1024)
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
                Ok(Err(_)) => Err("Extraction interrompue".to_string()),
                Err(_) => {
                    let mins = timeout_secs / 60;
                    Err(format!("Fichier trop complexe (timeout {mins}min)"))
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
        Ok(None)
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
    let mut deleted = deleted;
    let mut import_requested = import_requested;

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
                "Importer un document"
            }
            button {
                class: "w-full flex items-center gap-3 px-4 py-3 text-sm text-ios-red active:bg-stone-50",
                onclick: {
                    let note_id = note_id.clone();
                    move |_| {
                        app.show_note_menu.set(false);
                        deleted.set(true);
                        if let Ok(Some(n)) = db().get_note(&note_id) {
                            if let Some(ref f) = n.audio_file_path {
                                let _ = std::fs::remove_file(crate::services::audio::resolve_audio_path(f));
                            }
                        }
                        let _ = db().delete_note(&note_id);
                        delete_note_embeddings(note_id.clone());
                        app.notes_version.set((app.notes_version)() + 1);
                        app.sliding_out.set(true);
                        spawn(async move {
                            futures_timer::Delay::new(std::time::Duration::from_millis(150)).await;
                            app.sliding_out.set(false);
                            app.view.set(View::NotesList);
                        });
                    }
                },
                IconTrash { size: 18 }
                "Supprimer la note"
            }
        }
    }
}
