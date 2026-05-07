use crate::db::Database;
use crate::models::{NewTextNote, Note, NoteType, UpdateNote};
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn NotesList() -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();

    let notes = use_memo(move || {
        let db = db();
        match (app.selected_folder_id)() {
            Some(fid) => db.list_notes_in_folder(&fid).unwrap_or_default(),
            None => db.list_notes().unwrap_or_default(),
        }
    });

    rsx! {
        if notes().is_empty() {
            div { class: "flex-1 flex flex-col items-center justify-center gap-2 h-[60vh]",
                p { class: "text-lg text-gray-400", "Aucune note" }
                p { class: "text-sm text-gray-400", "Appuyez sur + pour commencer" }
            }
        } else {
            div {
                for note in notes() {
                    NoteCard { note: note }
                }
            }
        }
    }
}

#[component]
fn NoteCard(note: Note) -> Element {
    let mut app: AppState = use_context();
    let note_id = note.id.clone();

    let title = note
        .title
        .clone()
        .unwrap_or_else(|| "Sans titre".to_string());

    let preview = if note.content.len() > 120 {
        format!("{}...", &note.content[..120])
    } else {
        note.content.clone()
    };

    let date = &note.created_at[..10];

    let type_badge = match note.note_type {
        NoteType::Voice => "Vocale",
        NoteType::Text => "Texte",
    };

    rsx! {
        div {
            class: "bg-white p-4 border border-gray-200 rounded-xl mb-2.5",
            onclick: move |_| {
                app.view.set(View::NoteDetail { note_id: note_id.clone() });
            },
            div { class: "flex justify-between items-center mb-2",
                h3 { class: "font-semibold text-base text-gray-900", "{title}" }
                span { class: "text-xs px-2 py-0.5 rounded-full bg-gray-100 text-gray-400", "{type_badge}" }
            }
            if !preview.is_empty() {
                p { class: "text-gray-600 text-sm mb-2 line-clamp-2", "{preview}" }
            }
            p { class: "text-gray-400 text-xs", "{date}" }
        }
    }
}

#[component]
pub fn NoteDetail() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();

    let note_id = match (app.view)() {
        View::NoteDetail { note_id } => note_id,
        _ => return rsx! {},
    };

    let is_new = note_id.is_empty();

    let note = if is_new {
        None
    } else {
        db().get_note(&note_id).ok().flatten()
    };

    let initial_title = note
        .as_ref()
        .and_then(|n| n.title.clone())
        .unwrap_or_default();
    let initial_content =
        note.as_ref().map(|n| n.content.clone()).unwrap_or_default();

    let mut title = use_signal(|| initial_title.clone());
    let mut content = use_signal(|| initial_content.clone());

    let is_voice = note
        .as_ref()
        .map(|n| n.note_type == NoteType::Voice)
        .unwrap_or(false);

    let duration = note
        .as_ref()
        .and_then(|n| n.duration_secs)
        .map(|d| format!("{:.0}s", d))
        .unwrap_or_default();

    let date = note
        .as_ref()
        .map(|n| n.created_at[..10].to_string())
        .unwrap_or_default();

    rsx! {
        div { class: "flex flex-col gap-4 py-2",
            input {
                class: "text-xl font-semibold border-none outline-none py-2 text-gray-900 bg-transparent w-full",
                placeholder: "Titre de la note",
                value: "{title}",
                oninput: move |evt| title.set(evt.value()),
            }
            if is_voice {
                div { class: "flex gap-3",
                    span { class: "text-xs text-gray-400", "Vocale" }
                    if !duration.is_empty() {
                        span { class: "text-xs text-gray-400", "{duration}" }
                    }
                    if !date.is_empty() {
                        span { class: "text-xs text-gray-400", "{date}" }
                    }
                }
            } else if !date.is_empty() {
                p { class: "text-xs text-gray-400", "{date}" }
            }
            textarea {
                class: "w-full min-h-[200px] border border-gray-200 rounded-xl p-3 text-sm resize-y font-sans outline-none text-gray-900",
                placeholder: "Contenu de la note...",
                value: "{content}",
                oninput: move |evt| content.set(evt.value()),
            }
            button {
                class: "self-end px-6 py-2.5 rounded-full bg-ios-blue text-white text-sm font-medium",
                onclick: {
                    let note_id = note_id.clone();
                    move |_| {
                        let db = db();
                        if note_id.is_empty() {
                            let new = NewTextNote {
                                title: if title().is_empty() { None } else { Some(title()) },
                                content: content(),
                                tags: vec![],
                            };
                            let _ = db.create_text_note(&new);
                        } else {
                            let upd = UpdateNote {
                                title: Some(title()),
                                content: Some(content()),
                                tags: None,
                            };
                            let _ = db.update_note(&note_id, &upd);
                        }
                        app.view.set(View::NotesList);
                    }
                },
                "Enregistrer"
            }
        }
    }
}
