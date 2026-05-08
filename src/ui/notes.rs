use crate::db::Database;
use crate::models::{generate_auto_title, NewTextNote, Note, UpdateNote};
use crate::services::audio::{self, AudioRecorder, RecordingState};
use crate::services::transcription::SonioxClient;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn start_transcription(path: PathBuf, mut state: Signal<RecordingState>) {
    spawn(async move {
        let client = match SonioxClient::from_env() {
            Ok(c) => c,
            Err(e) => {
                state.set(RecordingState::Error(e));
                return;
            }
        };
        match client.transcribe(&path).await {
            Ok(text) => state.set(RecordingState::Transcribed(text)),
            Err(e) => state.set(RecordingState::Error(e)),
        }
    });
}

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
    let has_audio = note.audio_file_path.is_some();

    rsx! {
        div {
            class: "bg-white p-4 border border-gray-200 rounded-xl mb-2.5",
            onclick: move |_| {
                app.view.set(View::NoteDetail { note_id: note_id.clone() });
            },
            div { class: "flex justify-between items-center mb-2",
                h3 { class: "font-semibold text-base text-gray-900", "{title}" }
                if has_audio {
                    div { class: "w-2 h-2 rounded-full bg-ios-green" }
                }
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
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();

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

    let initial_title = if is_new {
        generate_auto_title()
    } else {
        note.as_ref()
            .and_then(|n| n.title.clone())
            .unwrap_or_default()
    };

    let initial_content =
        note.as_ref().map(|n| n.content.clone()).unwrap_or_default();

    let mut title = use_signal(|| initial_title.clone());
    let mut content = use_signal(|| initial_content.clone());
    let mut last_audio_path = use_signal(String::new);

    use_effect(move || {
        if let RecordingState::Transcribed(text) = (app.recording_state)() {
            let current = content();
            if current.is_empty() {
                content.set(text);
            } else {
                content.set(format!("{}\n{}", current, text));
            }
            app.recording_state.set(RecordingState::Idle);
        }
    });

    let recording_state = (app.recording_state)();
    let is_recording = recording_state == RecordingState::Recording;
    let is_transcribing = recording_state == RecordingState::Transcribing;

    let date = note
        .as_ref()
        .map(|n| n.created_at[..10].to_string())
        .unwrap_or_default();

    rsx! {
        div { class: "pb-20",
            input {
                class: "text-xl font-semibold border-none outline-none py-2 text-gray-900 bg-transparent w-full",
                placeholder: "Titre de la note",
                value: "{title}",
                oninput: move |evt| title.set(evt.value()),
            }
            if !date.is_empty() {
                p { class: "text-xs text-gray-400 mb-3", "{date}" }
            }
            textarea {
                class: "w-full min-h-[200px] border border-gray-200 rounded-xl p-3 text-sm resize-none font-sans outline-none text-gray-900",
                placeholder: "Contenu de la note...",
                value: "{content}",
                oninput: move |evt| content.set(evt.value()),
            }
            if is_recording {
                p { class: "text-sm text-ios-red text-center mt-3",
                    "Enregistrement..."
                }
            } else if is_transcribing {
                p { class: "text-sm text-gray-400 text-center mt-3",
                    "Transcription..."
                }
            } else if let RecordingState::Error(ref e) = recording_state {
                p { class: "text-sm text-ios-red text-center mt-3",
                    "Erreur : {e}"
                }
            }
        }
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-3 bg-white border-t border-gray-200 z-30",
            div { class: "flex items-center justify-between",
                button {
                    class: if is_recording {
                        "w-11 h-11 rounded-full bg-ios-red flex items-center justify-center"
                    } else if is_transcribing {
                        "w-11 h-11 rounded-full bg-gray-300 flex items-center justify-center"
                    } else {
                        "w-11 h-11 rounded-full bg-ios-green flex items-center justify-center"
                    },
                    disabled: is_transcribing,
                    onclick: move |_| {
                        let rec = recorder();
                        let mut rec = rec.lock().unwrap();
                        let current_state = (app.recording_state)();
                        match current_state {
                            RecordingState::Recording => {
                                match rec.stop(&audio::output_dir()) {
                                    Ok(path) => {
                                        last_audio_path.set(
                                            path.display().to_string(),
                                        );
                                        app.recording_state.set(
                                            RecordingState::Transcribing,
                                        );
                                        start_transcription(
                                            path,
                                            app.recording_state,
                                        );
                                    }
                                    Err(e) => {
                                        app.recording_state.set(
                                            RecordingState::Error(e),
                                        );
                                    }
                                }
                            }
                            _ => {
                                std::fs::create_dir_all(
                                    audio::output_dir(),
                                )
                                .ok();
                                match rec.start() {
                                    Ok(()) => {
                                        app.recording_state.set(
                                            RecordingState::Recording,
                                        );
                                    }
                                    Err(e) => {
                                        app.recording_state.set(
                                            RecordingState::Error(e),
                                        );
                                    }
                                }
                            }
                        }
                    },
                    if is_recording {
                        div { class: "w-4 h-4 rounded bg-white" }
                    } else {
                        div { class: "w-4 h-4 rounded-full bg-white" }
                    }
                }
                button {
                    class: "px-6 py-2.5 rounded-full bg-ios-blue text-white text-sm font-medium",
                    onclick: {
                        let note_id = note_id.clone();
                        move |_| {
                            let db = db();
                            if note_id.is_empty() {
                                let new = NewTextNote {
                                    title: if title().is_empty() {
                                        None
                                    } else {
                                        Some(title())
                                    },
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
                                let _ =
                                    db.update_note(&note_id, &upd);
                            }
                            app.view.set(View::NotesList);
                        }
                    },
                    "Enregistrer"
                }
            }
        }
    }
}
