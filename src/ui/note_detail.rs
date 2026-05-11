use crate::db::Database;
use crate::models::{generate_auto_title, Attachment, NewTextNote, UpdateNote};
use crate::services::audio::RecordingState;
use crate::services::embed::{
    delete_attachment_embeddings, delete_note_embeddings, embed_note,
};
use crate::services::llm::LlmClient;
use crate::ui::folder_picker::FolderPicker;
use crate::ui::icons::*;
use crate::ui::recording_bar::RecordingBar;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

struct ImportedFile {
    filename: String,
    content: String,
}

async fn import_file_content() -> Option<ImportedFile> {
    #[cfg(target_os = "ios")]
    {
        use crate::platform::ios::{open_file_picker, read_file_as_text};
        let paths =
            open_file_picker(&["txt", "md", "csv", "pdf", "docx"]).await?;
        let path = paths.first()?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document")
            .to_string();
        match read_file_as_text(path) {
            Ok(content) => Some(ImportedFile { filename, content }),
            Err(e) => {
                eprintln!("[import] {e}");
                None
            }
        }
    }
    #[cfg(not(target_os = "ios"))]
    {
        None
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
    let mut tags: Signal<Vec<String>> = use_signal(|| initial_tags.clone());
    let mut tag_input = use_signal(String::new);
    let mut tagging = use_signal(|| false);
    let mut deleted = use_signal(|| false);
    let mut confirm_delete_att: Signal<Option<String>> = use_signal(|| None);
    let mut local_note_id = use_signal(|| note_id.clone());

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
            if note_id.is_empty() && c.is_empty() {
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

    let recording_state = (app.recording_state)();

    let date = note
        .as_ref()
        .map(|n| n.created_at[..10].to_string())
        .unwrap_or_default();

    let show_menu = (app.show_note_menu)();

    rsx! {
        if show_menu {
            div {
                class: "fixed inset-0 z-40",
                onclick: move |_| app.show_note_menu.set(false),
            }
            div {
                class: "absolute right-4 top-1 z-50 bg-white rounded-xl shadow-lg border border-gray-100 py-1 min-w-[200px]",
                button {
                    class: "w-full flex items-center gap-3 px-4 py-3 text-sm text-gray-700 active:bg-gray-50",
                    onclick: move |_| {
                        app.show_note_menu.set(false);
                        let db = db();
                        let t = title();
                        let c = content();
                        let tg = tags();
                        let folder = selected_folder();
                        spawn(async move {
                            let Some(file) = import_file_content().await else {
                                return;
                            };
                            let target_id = {
                                let current = local_note_id();
                                if current.is_empty() {
                                    let new = NewTextNote {
                                        title: if t.is_empty() { None } else { Some(t.clone()) },
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
                                            eprintln!("[import] save note: {e}");
                                            return;
                                        }
                                    }
                                } else {
                                    current
                                }
                            };
                            let new_att = crate::models::NewAttachment {
                                note_id: target_id.clone(),
                                filename: file.filename.clone(),
                                content_text: file.content.clone(),
                            };
                            match db.create_attachment(&new_att) {
                                Ok(att) => {
                                    app.attachments_version.set((app.attachments_version)() + 1);
                                    crate::services::embed::embed_attachment(
                                        att.id.clone(),
                                        target_id.clone(),
                                        att.filename.clone(),
                                        att.content_text.clone(),
                                    );
                                }
                                Err(e) => eprintln!("[import] create attachment: {e}"),
                            }
                        });
                    },
                    IconFileArrowUp { size: 18 }
                    "Importer un document"
                }
                button {
                    class: "w-full flex items-center gap-3 px-4 py-3 text-sm text-ios-red active:bg-gray-50",
                    onclick: {
                        let note_id = note_id.clone();
                        move |_| {
                            app.show_note_menu.set(false);
                            deleted.set(true);
                            let _ = db().delete_note(&note_id);
                            delete_note_embeddings(note_id.clone());
                            app.notes_version.set((app.notes_version)() + 1);
                            app.sliding_out.set(true);
                            spawn(async move {
                                futures_timer::Delay::new(std::time::Duration::from_millis(150)).await;
                                app.sliding_out.set(false);
                                app.view.set(View::NotesList);
                            });
                        }
                    },
                    IconTrash { size: 18 }
                    "Supprimer la note"
                }
            }
        }
        div {
            class: "overflow-y-auto pb-20",
            style: "height: calc(100% - var(--keyboard-inset, 0px));",
            input {
                class: "text-xl font-semibold border-none outline-none py-2 text-gray-900 bg-transparent w-full",
                placeholder: "Titre de la note",
                value: "{title}",
                oninput: move |evt| title.set(evt.value()),
            }
            if !date.is_empty() {
                p { class: "text-xs text-gray-400 mb-2", "{date}" }
            }
            FolderPicker { selected: selected_folder }
            div { class: "flex flex-wrap items-center gap-1.5 mb-2",
                for (i, tag) in tags().iter().enumerate() {
                    {
                        let tag_display = tag.clone();
                        rsx! {
                            span {
                                key: "{i}",
                                class: "inline-flex items-center gap-1 px-2.5 py-1 rounded-full bg-ios-blue/10 text-ios-blue text-xs font-medium",
                                "{tag_display}"
                                button {
                                    class: "ml-0.5 text-ios-blue/60 hover:text-ios-blue",
                                    onclick: move |_| {
                                        let mut t = tags();
                                        t.remove(i);
                                        tags.set(t);
                                    },
                                    IconX { size: 12 }
                                }
                            }
                        }
                    }
                }
                div { class: "inline-flex items-center gap-1",
                    input {
                        class: "w-24 text-xs border border-gray-200 rounded-full px-2.5 py-1 outline-none",
                        placeholder: "+ tag",
                        value: "{tag_input}",
                        oninput: move |evt| tag_input.set(evt.value()),
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter {
                                let v = tag_input().trim().to_string();
                                if !v.is_empty() && !tags().contains(&v) {
                                    let mut t = tags();
                                    t.push(v);
                                    tags.set(t);
                                    tag_input.set(String::new());
                                }
                            }
                        },
                    }
                    button {
                        class: if tagging() {
                            "px-2.5 py-1 rounded-full bg-gray-100 text-gray-400 text-xs"
                        } else {
                            "px-2.5 py-1 rounded-full bg-ios-blue/10 text-ios-blue text-xs font-medium"
                        },
                        disabled: tagging() || content().trim().len() < 20,
                        onclick: move |_| {
                            tagging.set(true);
                            let c = content();
                            spawn(async move {
                                if let Ok(client) = LlmClient::from_env() {
                                    if let Ok(new_tags) =
                                        client.generate_tags(&c).await
                                    {
                                        let mut current = tags();
                                        for t in new_tags {
                                            if !current.contains(&t) {
                                                current.push(t);
                                            }
                                        }
                                        tags.set(current);
                                    }
                                }
                                tagging.set(false);
                            });
                        },
                        if tagging() { "..." } else { "Auto-tag" }
                    }
                }
            }
            textarea {
                class: "w-full min-h-[200px] border border-gray-200 rounded-xl p-3 text-sm resize-none font-sans outline-none text-gray-900",
                placeholder: "Contenu de la note...",
                value: "{content}",
                oninput: move |evt| content.set(evt.value()),
            }
            for att in attachments.iter() {
                {
                    let att_card = att.clone();
                    let att_name = att.filename.clone();
                    let att_name_confirm = att.filename.clone();
                    let att_id_del = att.id.clone();
                    let att_id_confirm = att.id.clone();
                    let date = att.imported_at[..10].to_string();
                    let is_confirming = confirm_delete_att() == Some(att.id.clone());
                    rsx! {
                        div {
                            key: "{att.id}",
                            class: "relative overflow-hidden bg-white border border-gray-200 rounded-xl h-[60px] flex items-center px-3 mt-2 active:bg-gray-50",
                            onclick: move |_| {
                                if !is_confirming {
                                    app.attachment_modal.set(Some(att_card.clone()));
                                }
                            },
                            div { class: "w-9 h-9 rounded-lg bg-blue-50 flex items-center justify-center flex-shrink-0",
                                IconFileArrowUp { size: 18 }
                            }
                            div { class: "flex-1 min-w-0 ml-3",
                                p { class: "text-sm font-medium text-gray-900 truncate leading-tight", "{att_name}" }
                                p { class: "text-xs text-gray-400 leading-tight", "{date}" }
                            }
                            button {
                                class: "w-11 h-11 flex items-center justify-center rounded-full text-gray-400 -mr-1 active:bg-gray-100",
                                onclick: {
                                    move |evt: Event<MouseData>| {
                                        evt.stop_propagation();
                                        confirm_delete_att.set(Some(att_id_confirm.clone()));
                                    }
                                },
                                IconTrash { size: 18 }
                            }
                            if is_confirming {
                                div {
                                    class: "absolute inset-0 z-10 flex items-center px-3 bg-white/95 backdrop-blur-sm",
                                    onclick: move |evt| evt.stop_propagation(),
                                    div { class: "flex-1 min-w-0 mr-3",
                                        p { class: "text-[10px] font-medium text-gray-400 uppercase tracking-wide leading-tight", "Supprimer ?" }
                                        p { class: "text-sm font-medium text-gray-600 truncate leading-tight", "{att_name_confirm}" }
                                    }
                                    div { class: "flex items-center gap-2",
                                        button {
                                            class: "h-9 px-4 flex items-center justify-center text-sm font-medium text-gray-900 bg-gray-100 rounded-full active:bg-gray-200",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                confirm_delete_att.set(None);
                                            },
                                            "Annuler"
                                        }
                                        button {
                                            class: "h-9 px-4 flex items-center justify-center text-sm font-medium text-white bg-ios-red rounded-full active:opacity-80",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                let db = db();
                                                let id = att_id_del.clone();
                                                let _ = db.delete_attachment(&id);
                                                delete_attachment_embeddings(id);
                                                confirm_delete_att.set(None);
                                                app.attachments_version.set((app.attachments_version)() + 1);
                                            },
                                            "Supprimer"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let RecordingState::Error(ref e) = recording_state {
                p { class: "text-xs text-gray-400 text-center mt-3",
                    "Erreur : {e}"
                }
            }
        }
        RecordingBar {}
    }
}
