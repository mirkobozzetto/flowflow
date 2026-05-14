use crate::db::Database;
use crate::models::{
    generate_auto_title, Attachment, NewAttachment, NewTextNote, UpdateNote,
};
use crate::services::audio::{self, RecordingState};
use crate::services::embed::{embed_attachment, embed_note};
use crate::ui::folder_picker::FolderPicker;
use crate::ui::notes::attachments::AttachmentSection;
use crate::ui::notes::audio_player::AudioPlayer;
use crate::ui::notes::menu::{import_file_content, NoteMenu};
use crate::ui::notes::tags::TagsSection;
use crate::ui::recording::RecordingBar;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn NoteDetail() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();

    let note_id = match (app.view)() {
        View::NoteDetail { note_id } => note_id,
        _ => return rsx! {},
    };

    app.current_note_id.set(Some(note_id.clone()));

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

    let initial_tags: Vec<String> =
        note.as_ref().map(|n| n.tags.clone()).unwrap_or_default();

    let mut title = use_signal(|| initial_title.clone());
    let mut content = use_signal(|| initial_content.clone());
    let selected_folder = use_signal(|| initial_folder_id.clone());
    let tags: Signal<Vec<String>> = use_signal(|| initial_tags.clone());
    let tag_input = use_signal(String::new);
    let tagging = use_signal(|| false);
    let deleted = use_signal(|| false);
    let confirm_delete_att: Signal<Option<String>> = use_signal(|| None);
    let mut local_note_id = use_signal(|| note_id.clone());
    let mut import_requested = use_signal(|| false);
    let mut import_status: Signal<Option<String>> = use_signal(|| None);
    let pending_audio: Signal<Option<(String, f64)>> = use_signal(|| None);
    let mut audio_state: Signal<Option<(String, f64)>> = use_signal(|| {
        note.as_ref().and_then(|n| {
            n.audio_file_path
                .as_ref()
                .map(|path| (path.clone(), n.duration_secs.unwrap_or(0.0)))
        })
    });

    use_effect(move || {
        let pa = pending_audio();
        if pa.is_some() {
            audio_state.set(pa);
        }
    });

    let attachments_version = (app.attachments_version)();
    let attachments: Vec<Attachment> = {
        let id = local_note_id();
        let _ = attachments_version;
        if id.is_empty() {
            Vec::new()
        } else {
            db().list_attachments_for_note(&id).unwrap_or_default()
        }
    };

    use_drop({
        let note_id = note_id.clone();
        move || {
            if deleted() {
                return;
            }
            let db = db();
            let t = title();
            let c = content();
            let pa = pending_audio();
            if note_id.is_empty() && c.is_empty() && pa.is_none() {
                return;
            }
            if !note_id.is_empty() && t.is_empty() && c.is_empty() {
                return;
            }
            let saved_id = if note_id.is_empty() {
                let new = NewTextNote {
                    title: if t.is_empty() { None } else { Some(t.clone()) },
                    content: c.clone(),
                    tags: tags(),
                    audio_file_path: pa.as_ref().map(|(p, _)| p.clone()),
                    duration_secs: pa.as_ref().map(|(_, d)| *d),
                };
                match db.create_text_note(&new) {
                    Ok(created) => {
                        if let Some(ref fid) = selected_folder() {
                            let _ = db.add_note_to_folder(&created.id, fid);
                        }
                        Some(created.id)
                    }
                    Err(_) => None,
                }
            } else {
                let upd = UpdateNote {
                    title: Some(t.clone()),
                    content: Some(c.clone()),
                    tags: Some(tags()),
                };
                let _ = db.update_note(&note_id, &upd);
                for old in db.folders_for_note(&note_id).unwrap_or_default() {
                    let _ = db.remove_note_from_folder(&note_id, &old.id);
                }
                if let Some(ref fid) = selected_folder() {
                    let _ = db.add_note_to_folder(&note_id, fid);
                }
                if let Some((p, d)) = pa {
                    let _ = db.update_audio_metadata(&note_id, &p, d);
                }
                Some(note_id.clone())
            };
            app.notes_version.set((app.notes_version)() + 1);
            if let Some(id) = saved_id {
                embed_note(id, t, c);
            }
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

    use_effect(move || {
        if !import_requested() {
            return;
        }
        import_requested.set(false);
        let db = db();
        let t = title();
        let c = content();
        let tg = tags();
        let folder = selected_folder();
        spawn(async move {
            import_status.set(Some("Importation en cours...".to_string()));
            let file = match import_file_content().await {
                Ok(Some(f)) => f,
                Ok(None) => {
                    import_status.set(None);
                    return;
                }
                Err(e) => {
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
                        audio_file_path: None,
                        duration_secs: None,
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
                            import_status
                                .set(Some(format!("Erreur sauvegarde: {e}")));
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
                    import_status.set(None);
                }
                Err(e) => {
                    import_status.set(Some(format!("Erreur import: {e}")));
                }
            }
        });
    });

    let recording_state = (app.recording_state)();
    let is_transcribing = recording_state == RecordingState::Transcribing;

    let date = note
        .as_ref()
        .map(|n| {
            if n.created_at.len() >= 16 {
                let d = &n.created_at[..10];
                let h = &n.created_at[11..16];
                format!("{d} · {h}")
            } else {
                n.created_at[..10].to_string()
            }
        })
        .unwrap_or_default();

    let show_menu = (app.show_note_menu)();

    rsx! {
        if show_menu {
            NoteMenu {
                note_id: note_id.clone(),
                import_requested,
                deleted,
            }
        }
        div {
            class: "overflow-y-auto pb-20",
            style: "height: calc(100% - var(--keyboard-inset, 0px));",
            input {
                class: "text-xl font-semibold border-none outline-none py-2 text-stone-900 bg-transparent w-full",
                placeholder: "Titre de la note",
                value: "{title}",
                oninput: move |evt| title.set(evt.value()),
            }
            if !date.is_empty() {
                p { class: "text-xs text-stone-400 mb-2", "{date}" }
            }
            FolderPicker { selected: selected_folder }
            TagsSection { tags, tag_input, tagging, content }
            textarea {
                class: if is_transcribing {
                    "w-full min-h-[200px] border border-ios-orange/30 rounded-xl p-3 text-sm resize-none font-sans outline-none text-stone-900"
                } else {
                    "w-full min-h-[200px] border border-stone-200 rounded-xl p-3 text-sm resize-none font-sans outline-none text-stone-900"
                },
                placeholder: if is_transcribing { "Transcription en cours..." } else { "Contenu de la note..." },
                value: "{content}",
                oninput: move |evt| content.set(evt.value()),
            }
            {
                if let Some((ref audio_filename, dur)) = audio_state() {
                    let resolved = audio::resolve_audio_path(audio_filename);
                    let note_id_clone = note_id.clone();
                    rsx! {
                        AudioPlayer {
                            audio_path: resolved,
                            duration_secs: Some(dur),
                            on_delete: move |_| {
                                if let Some((ref f, _)) = audio_state() {
                                    let _ = std::fs::remove_file(audio::resolve_audio_path(f));
                                }
                                let _ = db().clear_audio_metadata(&note_id_clone);
                                audio_state.set(None);
                                app.notes_version.set((app.notes_version)() + 1);
                            },
                        }
                    }
                } else {
                    rsx! {}
                }
            }
            AttachmentSection { attachments, confirm_delete_att }
            if let Some(ref status) = import_status() {
                div { class: if status.starts_with("Importation") {
                        "flex items-center gap-2 px-3 py-2 mt-2 bg-ios-orange/10 rounded-lg"
                    } else {
                        "flex items-center gap-2 px-3 py-2 mt-2 bg-ios-red/10 rounded-lg"
                    },
                    if status.starts_with("Importation") {
                        span { class: "inline-block w-3 h-3 border-2 border-ios-orange border-t-transparent rounded-full animate-spin" }
                    }
                    span { class: "text-xs text-stone-600", "{status}" }
                }
            }
            if let RecordingState::Error(ref e) = recording_state {
                p { class: "text-xs text-stone-400 text-center mt-3",
                    "Erreur : {e}"
                }
            }
        }
        RecordingBar { pending_audio }
    }
}
