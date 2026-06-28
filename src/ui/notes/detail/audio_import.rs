use crate::application::transcription_manager::TranscriptionManager;
use crate::domain::NewTextNote;
use crate::infrastructure::persistence::Database;
use crate::ui::notes::menu::import_audio_file;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

pub fn use_audio_import(
    mut app: AppState,
    db: Signal<Arc<Database>>,
    manager: TranscriptionManager,
    title: Signal<String>,
    content: Signal<String>,
    tags: Signal<Vec<String>>,
    mut local_note_id: Signal<String>,
) {
    let enqueue_manager = manager.clone();
    use_effect(move || {
        if !(app.audio_import_requested)() {
            return;
        }
        app.audio_import_requested.set(false);
        let manager = enqueue_manager.clone();
        let db = db();
        let t = title();
        let c = content();
        let tg = tags();
        let folder = (app.detail_folder_id)();
        spawn(async move {
            let path = match import_audio_file().await {
                Some(p) => p,
                None => return,
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
                            app.notes_version.set((app.notes_version)() + 1);
                            created.id
                        }
                        Err(_) => return,
                    }
                } else {
                    current
                }
            };
            manager.enqueue(target_id, path);
        });
    });
}
