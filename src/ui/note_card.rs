use crate::db::Database;
use crate::models::Note;
use crate::services::i18n::t;
use crate::ui::icons::IconBell;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn NoteCard(note: Note) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let note_id = note.id.clone();
    let lang = (app.current_lang)();

    let title = note
        .title
        .clone()
        .unwrap_or_else(|| t(&lang, "note-card-untitled"));

    let preview = if note.content.len() > 120 {
        format!("{}...", &note.content[..120])
    } else {
        note.content.clone()
    };

    let date = &note.created_at[..10];
    let _ = (app.notes_version)();
    let has_audio = db().has_any_audio(&note.id);
    let active_reminders: Vec<_> = db()
        .reminders_for_note(&note.id)
        .into_iter()
        .filter(|r| r.state == "active")
        .collect();
    let has_reminder = !active_reminders.is_empty();
    let reminder_due = active_reminders
        .first()
        .map(|r| crate::services::i18n::reminder_due_label(&lang, r));

    let folder_name = db()
        .folders_for_note(&note.id)
        .ok()
        .and_then(|f| f.first().map(|f| f.name.clone()));

    rsx! {
        div {
            class: "bg-warm-white p-4 border border-stone-200 rounded-xl mb-2.5 lg:mb-0 cursor-pointer hover:border-stone-300 hover:shadow-sm transition-all duration-150",
            onclick: move |_| {
                app.show_folder_picker.set(false);
                app.view.set(View::NoteDetail { note_id: note_id.clone() });
            },
            div { class: "flex justify-between items-center mb-2",
                h3 { class: "font-semibold text-base text-stone-900", "{title}" }
                div { class: "flex items-center gap-1.5 shrink-0",
                    if has_reminder {
                        span { class: "text-ios-orange-dark", IconBell { size: 14 } }
                    }
                    if has_audio {
                        div { class: "w-2 h-2 rounded-full bg-ios-orange/50" }
                    }
                }
            }
            if !preview.is_empty() {
                p { class: "text-stone-600 text-sm mb-2 line-clamp-2", "{preview}" }
            }
            if !note.tags.is_empty() {
                div { class: "flex flex-wrap gap-1 mb-1.5",
                    for tag in note.tags.iter() {
                        span { class: "px-2 py-0.5 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-xs font-medium",
                            "{tag}"
                        }
                    }
                }
            }
            div { class: "flex items-center gap-2",
                span { class: "text-stone-500 text-xs", "{date}" }
                if let Some(ref fname) = folder_name {
                    span { class: "text-xs text-stone-400", "·" }
                    span { class: "text-xs text-ios-orange-dark", "{fname}" }
                }
            }
            if let Some(ref due) = reminder_due {
                div { class: "flex items-center gap-1 mt-1 text-ios-orange-dark text-xs font-medium",
                    IconBell { size: 12 }
                    span { "{due}" }
                }
            }
        }
    }
}
