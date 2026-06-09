use crate::db::conflict_repo::SyncConflict;
use crate::db::Database;
use crate::services::i18n::t;
use crate::services::sync::conflict::{
    dismiss_conflict, restore_note_conflict,
};
use crate::services::sync::engine::SyncEngine;
use crate::services::sync::reconcile::spawn_reconcile_pass;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

fn snapshot_field(c: &SyncConflict, key: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(&c.losing_snapshot_json)
        .ok()?
        .get(key)?
        .as_str()
        .map(|s| s.to_string())
}

fn preview(c: &SyncConflict) -> String {
    let text = snapshot_field(c, "content")
        .or_else(|| snapshot_field(c, "name"))
        .or_else(|| snapshot_field(c, "content_text"))
        .unwrap_or_else(|| c.losing_snapshot_json.clone());
    let mut p: String = text.chars().take(120).collect();
    if p.len() < text.len() {
        p.push('…');
    }
    p
}

fn label(c: &SyncConflict) -> String {
    snapshot_field(c, "title")
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| format!("{} {}", c.entity_kind, c.entity_id))
}

// Unresolved sync conflicts (RFC T18): the losing version of every concurrent
// edit, kept until the user decides. A note conflict can come back as a new
// note (snapshot + archived vectors, zero re-embedding); anything can be
// dismissed once read.
#[component]
pub fn SyncConflicts() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let engine: Signal<Arc<SyncEngine>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();

    let mut version = use_signal(|| 0u32);
    let conflicts = use_memo(move || {
        version();
        db.peek().list_unresolved_conflicts().unwrap_or_default()
    });

    use_future(move || async move {
        let mut last = usize::MAX;
        loop {
            let count = db.peek().count_unresolved_conflicts().unwrap_or(0);
            if count != last {
                last = count;
                version += 1;
            }
            futures_timer::Delay::new(std::time::Duration::from_millis(2000))
                .await;
        }
    });

    if conflicts().is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "border-t border-stone-200 pt-4",
            h2 { class: "text-lg font-semibold text-stone-900 mb-1",
                {t(&lang, "sync-conflicts-title")}
            }
            p { class: "text-xs text-stone-500 mb-3 leading-relaxed",
                {t(&lang, "sync-conflicts-hint")}
            }
            div { class: "space-y-2",
                for c in conflicts() {
                    {
                        let conflict = c.clone();
                        let dismiss_id = c.id;
                        let mut app = app;
                        rsx! {
                            div {
                                key: "{c.id}",
                                class: "bg-warm-white border border-stone-200 rounded-xl px-3 py-2.5 space-y-1",
                                p { class: "text-sm text-stone-900 font-medium truncate",
                                    {label(&c)}
                                }
                                p { class: "text-xs text-stone-500 break-words",
                                    {preview(&c)}
                                }
                                div { class: "flex gap-4 pt-1",
                                    if c.entity_kind == "note" {
                                        button {
                                            class: "text-xs text-ios-orange font-medium min-h-[44px]",
                                            onclick: move |_| {
                                                match restore_note_conflict(&db.peek(), &conflict) {
                                                    Ok(_) => {
                                                        app.notes_version.set((app.notes_version)() + 1);
                                                        version += 1;
                                                        spawn_reconcile_pass();
                                                        engine.peek().schedule_debounced();
                                                    }
                                                    Err(e) => eprintln!("[sync] restore conflict: {e}"),
                                                }
                                            },
                                            {t(&lang, "sync-conflict-restore")}
                                        }
                                    }
                                    button {
                                        class: "text-xs text-stone-500 font-medium min-h-[44px]",
                                        onclick: move |_| {
                                            if let Err(e) = dismiss_conflict(&db.peek(), dismiss_id) {
                                                eprintln!("[sync] dismiss conflict: {e}");
                                            }
                                            version += 1;
                                        },
                                        {t(&lang, "sync-conflict-dismiss")}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
