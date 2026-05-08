use crate::db::Database;
use crate::models::{
    generate_auto_title, Folder, NewTextNote, Note, UpdateNote,
};
use crate::services::audio::{self, AudioRecorder, RecordingState};
use crate::services::transcription::SonioxClient;
use crate::ui::icons::*;
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
        let _v = (app.notes_version)();
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
    let db: Signal<Arc<Database>> = use_context();
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

    let folder_name = db()
        .folders_for_note(&note.id)
        .ok()
        .and_then(|f| f.first().map(|f| f.name.clone()));

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
            div { class: "flex items-center gap-2",
                span { class: "text-gray-400 text-xs", "{date}" }
                if let Some(ref fname) = folder_name {
                    span { class: "text-xs text-gray-400", "·" }
                    span { class: "text-xs text-ios-blue", "{fname}" }
                }
            }
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

    let initial_folder_id: Option<String> = if is_new {
        (app.selected_folder_id)()
    } else {
        db().folders_for_note(&note_id)
            .ok()
            .and_then(|f| f.first().map(|f| f.id.clone()))
    };

    let mut title = use_signal(|| initial_title.clone());
    let mut content = use_signal(|| initial_content.clone());
    let mut last_audio_path = use_signal(String::new);
    let mut selected_folder = use_signal(|| initial_folder_id.clone());
    let mut show_picker = use_signal(|| false);
    let mut deleted = use_signal(|| false);

    let all_folders: Memo<Vec<Folder>> = use_memo(move || {
        let _v = (app.folders_version)();
        db().list_all_folders().unwrap_or_default()
    });

    let folder_display = match selected_folder() {
        Some(ref fid) => all_folders()
            .iter()
            .find(|f| f.id == *fid)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "Dossier".to_string()),
        None => "Aucun dossier".to_string(),
    };

    use_effect(move || {
        let state = (app.recording_state)();
        if state == RecordingState::Recording {
            let rec = recorder();
            spawn(async move {
                loop {
                    if (app.recording_state)() != RecordingState::Recording {
                        app.audio_levels.set(vec![0.0; 12]);
                        break;
                    }
                    let levels = rec.lock().unwrap().current_levels(12);
                    app.audio_levels.set(levels);
                    futures_timer::Delay::new(
                        std::time::Duration::from_millis(80),
                    )
                    .await;
                }
            });
        }
    });

    use_drop({
        let note_id = note_id.clone();
        move || {
            if deleted() {
                return;
            }
            let db = db();
            let t = title();
            let c = content();
            if t.is_empty() && c.is_empty() {
                return;
            }
            if note_id.is_empty() {
                let new = NewTextNote {
                    title: if t.is_empty() { None } else { Some(t) },
                    content: c,
                    tags: vec![],
                };
                if let Ok(created) = db.create_text_note(&new) {
                    if let Some(ref fid) = selected_folder() {
                        let _ = db.add_note_to_folder(&created.id, fid);
                    }
                }
            } else {
                let upd = UpdateNote {
                    title: Some(t),
                    content: Some(c),
                    tags: None,
                };
                let _ = db.update_note(&note_id, &upd);
                for old in db.folders_for_note(&note_id).unwrap_or_default() {
                    let _ = db.remove_note_from_folder(&note_id, &old.id);
                }
                if let Some(ref fid) = selected_folder() {
                    let _ = db.add_note_to_folder(&note_id, fid);
                }
            }
            app.notes_version.set((app.notes_version)() + 1);
        }
    });

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
                p { class: "text-xs text-gray-400 mb-2", "{date}" }
            }
            div { class: "mb-3",
                button {
                    class: "inline-flex items-center px-3 py-1.5 rounded-full border border-gray-200 text-xs text-gray-600",
                    onclick: move |_| show_picker.set(!show_picker()),
                    "{folder_display}"
                }
                if show_picker() {
                    div { class: "mt-1 border border-gray-200 rounded-xl bg-white overflow-hidden",
                        button {
                            class: "w-full text-left px-3 py-2.5 text-sm text-gray-500 border-b border-gray-100",
                            onclick: move |_| {
                                selected_folder.set(None);
                                show_picker.set(false);
                            },
                            "Aucun dossier"
                        }
                        for folder in all_folders() {
                            {
                                let fid = folder.id.clone();
                                let fname = folder.name.clone();
                                let is_current = selected_folder() == Some(fid.clone());
                                rsx! {
                                    button {
                                        class: "w-full text-left px-3 py-2.5 text-sm",
                                        class: if is_current { "text-ios-blue font-medium bg-blue-50" } else { "text-gray-900" },
                                        onclick: move |_| {
                                            selected_folder.set(Some(fid.clone()));
                                            show_picker.set(false);
                                        },
                                        "{fname}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            textarea {
                class: "w-full min-h-[200px] border border-gray-200 rounded-xl p-3 text-sm resize-none font-sans outline-none text-gray-900",
                placeholder: "Contenu de la note...",
                value: "{content}",
                oninput: move |evt| content.set(evt.value()),
            }
            if let RecordingState::Error(ref e) = recording_state {
                p { class: "text-xs text-gray-400 text-center mt-3",
                    "Erreur : {e}"
                }
            }
            if !is_new {
                div { class: "mt-6 pt-4 border-t border-gray-100",
                    button {
                        class: "flex items-center gap-1.5 text-xs text-gray-400",
                        onclick: {
                            let note_id = note_id.clone();
                            move |_| {
                                deleted.set(true);
                                let _ = db().delete_note(&note_id);
                                app.notes_version.set((app.notes_version)() + 1);
                                app.sliding_out.set(true);
                                spawn(async move {
                                    futures_timer::Delay::new(std::time::Duration::from_millis(150)).await;
                                    app.sliding_out.set(false);
                                    app.view.set(View::NotesList);
                                });
                            }
                        },
                        IconTrash { size: 14 }
                        "Supprimer"
                    }
                }
            }
        }
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-3 bg-white border-t border-gray-200 z-30",
            div { class: "flex items-center justify-center",
                button {
                    class: if is_transcribing {
                        "w-full flex items-center justify-center gap-2 h-12 rounded-full bg-gray-100 text-gray-400 text-sm"
                    } else if is_recording {
                        "w-full flex items-center justify-center gap-3 h-12 rounded-full bg-gray-900 text-white text-sm shadow-lg"
                    } else {
                        "w-full flex items-center justify-center gap-2.5 h-12 rounded-full bg-ios-blue/10 text-ios-blue text-sm font-medium"
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
                        div { class: "w-2 h-2 rounded-full bg-ios-red" }
                        {
                            let levels = (app.audio_levels)();
                            rsx! {
                                div { class: "flex items-end gap-[3px] h-5",
                                    for (i, &lvl) in levels.iter().enumerate() {
                                        {
                                            let h = 3.0 + lvl * 17.0;
                                            let key = format!("bar-{i}");
                                            rsx! {
                                                div {
                                                    key: "{key}",
                                                    class: "w-[3px] bg-white rounded-full transition-all duration-75",
                                                    style: "height: {h:.0}px;",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        span { "Stop" }
                    } else if is_transcribing {
                        span { style: "animation: pulseSoft 1.5s ease-in-out infinite;", "Transcription..." }
                    } else {
                        IconMic { size: 22 }
                        span { "Dicter" }
                    }
                }
            }
        }
    }
}
