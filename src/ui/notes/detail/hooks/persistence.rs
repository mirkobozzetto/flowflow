use crate::application::embed::embed_note;
use crate::application::note_persistence::{create_note, update_note};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::engine::SyncEngine;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

/// Finish a committed deletion without redirecting a newer view.
pub fn use_delete_exit(
    mut app: AppState,
    local_note_id: Signal<String>,
    deleted: Signal<bool>,
) {
    use_effect(move || {
        if deleted() {
            let deleted_id = local_note_id.peek().clone();
            app.sliding_out.set(true);
            // spawn_forever: NoteDetail can unmount mid-delay (e.g. a sync-driven
            // rerender); a cancelled scope task would leave sliding_out stuck true.
            dioxus::core::spawn_forever(async move {
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    150,
                ))
                .await;
                app.sliding_out.set(false);
                if matches!((app.view)(), crate::ui::View::NoteDetail { note_id } if note_id == deleted_id)
                {
                    app.view.set(crate::ui::View::NotesList);
                }
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub fn use_save_on_drop(
    mut app: AppState,
    db: Signal<Arc<Database>>,
    engine: Signal<Arc<SyncEngine>>,
    title: Signal<String>,
    content: Signal<String>,
    tags: Signal<Vec<String>>,
    local_note_id: Signal<String>,
    deleted: Signal<bool>,
    pending_audio: Signal<Option<(String, f64)>>,
    base_title: Signal<String>,
    base_content: Signal<String>,
    base_tags: Signal<Vec<String>>,
    initial_folder_id: Option<String>,
) {
    use_drop({
        let orig_folder = initial_folder_id.clone();
        move || {
            if deleted() {
                return;
            }
            let db = db();
            let t = title();
            let c = content();
            let pa = pending_audio();
            let nid = local_note_id();
            // A reopened detail can outlive an in-flight deletion from its predecessor.
            if !nid.is_empty() && matches!(db.get_note(&nid), Ok(None)) {
                return;
            }
            if nid.is_empty() && c.is_empty() && pa.is_none() {
                return;
            }
            if !nid.is_empty() && t.is_empty() && c.is_empty() {
                return;
            }
            let title_changed = t != *base_title.peek();
            let content_changed = c != *base_content.peek();
            let tags_changed = tags() != *base_tags.peek();
            let folder = (app.detail_folder_id)();
            let folder_changed = folder != orig_folder;
            let has_new_audio = pa.is_some();
            let changed = title_changed
                || content_changed
                || tags_changed
                || folder_changed
                || has_new_audio;
            if !nid.is_empty() && !changed {
                return;
            }
            let (saved_id, saved_created_at) = if nid.is_empty() {
                match create_note(&db, &t, &c, tags(), folder.as_deref(), pa) {
                    Some((created, _)) => {
                        (Some(created.id), created.created_at)
                    }
                    None => (None, String::new()),
                }
            } else {
                let ca =
                    update_note(&db, &nid, &t, &c, tags(), folder.as_deref());
                (Some(nid.clone()), ca)
            };
            app.notes_version.set((app.notes_version)() + 1);
            if let Some(id) = &saved_id {
                embed_note(
                    id.clone(),
                    t.clone(),
                    c.clone(),
                    tags(),
                    saved_created_at,
                );
                engine.peek().schedule_debounced();
                // A note saved into a shared theme has to reach the space too,
                // otherwise it stays local and no member ever sees it. Detached
                // because this runs on unmount: no async context survives here.
                let space_note = id.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async move {
                        let Ok(database) =
                            crate::infrastructure::persistence::Database::open(
                            )
                        else {
                            return;
                        };
                        if let Err(e) =
                            crate::application::space::publish_local_note(
                                &database,
                                &space_note,
                            )
                            .await
                        {
                            eprintln!("[space] publish {space_note}: {e}");
                        }
                    });
                });
            }
        }
    });
}
