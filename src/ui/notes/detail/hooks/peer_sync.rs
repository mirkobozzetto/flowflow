use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub fn use_peer_merge(
    app: AppState,
    db: Signal<Arc<Database>>,
    local_note_id: Signal<String>,
    deleted: Signal<bool>,
    mut title: Signal<String>,
    mut content: Signal<String>,
    mut tags: Signal<Vec<String>>,
    mut base_title: Signal<String>,
    mut base_content: Signal<String>,
    mut base_tags: Signal<Vec<String>>,
    mut updated_from_peer: Signal<bool>,
) {
    use_effect(move || {
        let _v = (app.sync_data_version)();
        let id = local_note_id();
        if id.is_empty() || deleted() {
            return;
        }
        let Some(fresh) = db().get_note(&id).ok().flatten() else {
            return;
        };
        let fresh_title = fresh.title.clone().unwrap_or_default();
        let changed = fresh_title != *base_title.peek()
            || fresh.content != *base_content.peek()
            || fresh.tags != *base_tags.peek();
        if !changed {
            return;
        }
        let dirty = title.peek().as_str() != base_title.peek().as_str()
            || content.peek().as_str() != base_content.peek().as_str()
            || *tags.peek() != *base_tags.peek();
        if dirty {
            updated_from_peer.set(true);
            return;
        }
        title.set(fresh_title.clone());
        content.set(fresh.content.clone());
        tags.set(fresh.tags.clone());
        base_title.set(fresh_title);
        base_content.set(fresh.content);
        base_tags.set(fresh.tags);
    });
}
