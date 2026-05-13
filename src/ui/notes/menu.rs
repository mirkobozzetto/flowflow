use crate::db::Database;
use crate::models::NewTextNote;
use crate::services::embed::delete_note_embeddings;
use crate::ui::icons::*;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

pub struct ImportedFile {
    pub filename: String,
    pub content: String,
}

pub async fn import_file_content() -> Option<ImportedFile> {
    #[cfg(target_os = "ios")]
    {
        use crate::platform::ios::{open_file_picker, read_file_as_text};
        let paths =
            open_file_picker(&["txt", "md", "csv", "pdf", "docx"]).await?;
        let path = paths.first()?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document")
            .to_string();
        match read_file_as_text(path) {
            Ok(content) => Some(ImportedFile { filename, content }),
            Err(e) => {
                eprintln!("[import] {e}");
                None
            }
        }
    }
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

#[component]
pub fn NoteMenu(
    note_id: String,
    title: Signal<String>,
    content: Signal<String>,
    tags: Signal<Vec<String>>,
    selected_folder: Signal<Option<String>>,
    local_note_id: Signal<String>,
    deleted: Signal<bool>,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let title = title;
    let content = content;
    let tags = tags;
    let selected_folder = selected_folder;
    let mut local_note_id = local_note_id;
    let mut deleted = deleted;

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
                    app.show_note_menu.set(false);
                    let db = db();
                    let t = title();
                    let c = content();
                    let tg = tags();
                    let folder = selected_folder();
                    spawn(async move {
                        let Some(file) = import_file_content().await else {
                            return;
                        };
                        let target_id = {
                            let current = local_note_id();
                            if current.is_empty() {
                                let new = NewTextNote {
                                    title: if t.is_empty() { None } else { Some(t.clone()) },
                                    content: c.clone(),
                                    tags: tg.clone(),
                                };
                                match db.create_text_note(&new) {
                                    Ok(created) => {
                                        if let Some(ref fid) = folder {
                                            let _ = db.add_note_to_folder(&created.id, fid);
                                        }
                                        local_note_id.set(created.id.clone());
                                        app.current_note_id.set(Some(created.id.clone()));
                                        created.id
                                    }
                                    Err(e) => {
                                        eprintln!("[import] save note: {e}");
                                        return;
                                    }
                                }
                            } else {
                                current
                            }
                        };
                        let new_att = crate::models::NewAttachment {
                            note_id: target_id.clone(),
                            filename: file.filename.clone(),
                            content_text: file.content.clone(),
                        };
                        match db.create_attachment(&new_att) {
                            Ok(att) => {
                                app.attachments_version.set((app.attachments_version)() + 1);
                                crate::services::embed::embed_attachment(
                                    att.id.clone(),
                                    target_id.clone(),
                                    att.filename.clone(),
                                    att.content_text.clone(),
                                );
                            }
                            Err(e) => eprintln!("[import] create attachment: {e}"),
                        }
                    });
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
