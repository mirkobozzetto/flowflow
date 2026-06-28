use crate::application::embed::embed_attachment;
use crate::application::i18n::t_args;
use crate::domain::{NewAttachment, NewTextNote};
use crate::infrastructure::persistence::Database;
use crate::ui::notes::menu::import_file_content;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn use_document_import(
    mut app: AppState,
    db: Signal<Arc<Database>>,
    title: Signal<String>,
    content: Signal<String>,
    tags: Signal<Vec<String>>,
    mut local_note_id: Signal<String>,
    mut import_requested: Signal<bool>,
    mut import_in_progress: Signal<bool>,
    mut import_status: Signal<Option<String>>,
) {
    use_effect(move || {
        if !import_requested() {
            return;
        }
        import_requested.set(false);
        let db = db();
        let t = title();
        let c = content();
        let tg = tags();
        let folder = (app.detail_folder_id)();
        let lang_eff = (app.current_lang)();
        spawn(async move {
            import_in_progress.set(true);
            import_status.set(Some(t_args(
                &lang_eff,
                "note-import-in-progress",
                &[],
            )));
            let file = match import_file_content(&lang_eff).await {
                Ok(Some(f)) => f,
                Ok(None) => {
                    import_in_progress.set(false);
                    import_status.set(None);
                    return;
                }
                Err(e) => {
                    import_in_progress.set(false);
                    import_status.set(Some(e));
                    spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(4))
                            .await;
                        import_status.set(None);
                    });
                    return;
                }
            };
            let target_id = {
                let current = local_note_id();
                if current.is_empty() {
                    let new = NewTextNote {
                        title: if t.is_empty() {
                            None
                        } else {
                            Some(t.clone())
                        },
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
                            import_in_progress.set(false);
                            let msg = e.to_string();
                            import_status.set(Some(t_args(
                                &lang_eff,
                                "note-import-save-error",
                                &[("error", &msg)],
                            )));
                            return;
                        }
                    }
                } else {
                    current
                }
            };
            let new_att = NewAttachment {
                note_id: target_id.clone(),
                filename: file.filename.clone(),
                content_text: file.content.clone(),
            };
            match db.create_attachment(&new_att) {
                Ok(att) => {
                    app.attachments_version
                        .set((app.attachments_version)() + 1);
                    embed_attachment(
                        att.id.clone(),
                        target_id.clone(),
                        att.filename.clone(),
                        att.content_text.clone(),
                    );
                    import_in_progress.set(false);
                    import_status.set(None);
                }
                Err(e) => {
                    import_in_progress.set(false);
                    let msg = e.to_string();
                    import_status.set(Some(t_args(
                        &lang_eff,
                        "note-import-error",
                        &[("error", &msg)],
                    )));
                }
            }
        });
    });
}
