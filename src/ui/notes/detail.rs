use crate::db::Database;
use crate::models::{
    generate_auto_title, is_auto_title, Attachment, NewAttachment, NewTextNote,
    UpdateNote,
};
use crate::services::audio::RecordingState;
use crate::services::embed::{embed_attachment, embed_note};
use crate::services::i18n::{t, t_args};
use crate::ui::icons::{IconCheck, IconCopy};
use crate::ui::notes::attachments::AttachmentSection;
use crate::ui::notes::audio_section::{AudioJobBanner, AudioSection};
use crate::ui::notes::dates::{format_absolute_short, format_relative_date};
use crate::ui::notes::menu::{
    import_audio_file, import_file_content, NoteMenu,
};
use crate::ui::notes::reminders::{ActiveReminders, ReminderSuggestions};
use crate::ui::notes::tags::TagsSection;
use crate::ui::recording::RecordingBar;
use crate::ui::transcription_manager::TranscriptionManager;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn NoteDetail() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let engine: Signal<Arc<crate::services::sync::engine::SyncEngine>> =
        use_context();
    let manager: TranscriptionManager = use_context();
    let lang = (app.current_lang)();

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
        generate_auto_title(&lang)
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
    let folder_init = use_signal(|| {
        app.detail_folder_id.set(initial_folder_id.clone());
        true
    });
    let _ = folder_init();

    let initial_tags: Vec<String> =
        note.as_ref().map(|n| n.tags.clone()).unwrap_or_default();

    let mut title = use_signal(|| initial_title.clone());
    let mut content = use_signal(|| initial_content.clone());
    let mut tags: Signal<Vec<String>> = use_signal(|| initial_tags.clone());
    let mut base_title = use_signal(|| initial_title.clone());
    let mut base_content = use_signal(|| initial_content.clone());
    let mut base_tags: Signal<Vec<String>> =
        use_signal(|| initial_tags.clone());
    let mut updated_from_peer = use_signal(|| false);
    let tag_input = use_signal(String::new);
    let tagging = use_signal(|| false);
    let deleted = use_signal(|| false);
    let mut generating_title = use_signal(|| false);
    let mut title_gen_done = use_signal(|| false);
    let confirm_delete_att: Signal<Option<String>> = use_signal(|| None);
    let mut local_note_id = use_signal(|| note_id.clone());
    let mut import_requested = use_signal(|| false);
    let mut import_status: Signal<Option<String>> = use_signal(|| None);
    let mut import_in_progress = use_signal(|| false);
    let mut note_copied = use_signal(|| false);
    let mut pending_audio: Signal<Option<(String, f64)>> = use_signal(|| None);
    let mut audios_version = use_signal(|| 0u32);

    use_effect(move || {
        if !deleted() {
            app.current_note_id.set(Some(local_note_id()));
        }
    });

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

    use_effect(move || {
        if deleted() {
            app.sliding_out.set(true);
            spawn(async move {
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    150,
                ))
                .await;
                app.sliding_out.set(false);
                app.notes_version.set((app.notes_version)() + 1);
                app.view.set(View::NotesList);
            });
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
            if nid.is_empty() && c.is_empty() && pa.is_none() {
                return;
            }
            if !nid.is_empty() && t.is_empty() && c.is_empty() {
                return;
            }
            let title_changed = t != *base_title.peek();
            let content_changed = c != *base_content.peek();
            let tags_changed = tags() != *base_tags.peek();
            let folder_changed = (app.detail_folder_id)() != orig_folder;
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
                let new = NewTextNote {
                    title: if t.is_empty() { None } else { Some(t.clone()) },
                    content: c.clone(),
                    tags: tags(),
                };
                match db.create_text_note(&new) {
                    Ok(created) => {
                        if let Some(ref fid) = (app.detail_folder_id)() {
                            let _ = db.add_note_to_folder(&created.id, fid);
                        }
                        if let Some((p, d)) = pa {
                            let _ = db.add_audio(&created.id, &p, d);
                        }
                        let ca = created.created_at.clone();
                        (Some(created.id), ca)
                    }
                    Err(_) => (None, String::new()),
                }
            } else {
                let upd = UpdateNote {
                    title: Some(t.clone()),
                    content: Some(c.clone()),
                    tags: Some(tags()),
                };
                let _ = db.update_note(&nid, &upd);
                for old in db.folders_for_note(&nid).unwrap_or_default() {
                    let _ = db.remove_note_from_folder(&nid, &old.id);
                }
                if let Some(ref fid) = (app.detail_folder_id)() {
                    let _ = db.add_note_to_folder(&nid, fid);
                }
                let ca = db
                    .get_note(&nid)
                    .ok()
                    .flatten()
                    .map(|n| n.created_at)
                    .unwrap_or_default();
                (Some(nid.clone()), ca)
            };
            app.notes_version.set((app.notes_version)() + 1);
            if let Some(ref id) = saved_id {
                embed_note(
                    id.clone(),
                    t.clone(),
                    c.clone(),
                    tags(),
                    saved_created_at,
                );
                engine.peek().schedule_debounced();
            }
        }
    });

    use_effect(move || {
        if let RecordingState::Transcribed(text) = (app.recording_state)() {
            let current = content();
            if current.is_empty() {
                content.set(text.clone());
            } else {
                content.set(format!("{}\n{}", current, text));
            }
            let id = local_note_id();
            if !id.is_empty() {
                if let Some(last) =
                    db().list_audios(&id).ok().and_then(|a| a.last().cloned())
                {
                    if last.transcription.is_none() {
                        let _ = db().set_audio_transcription(&last.id, &text);
                        audios_version.set(audios_version() + 1);
                    }
                }
            }
            app.recording_state.set(RecordingState::Idle);
        }
    });

    use_effect(move || {
        let c = content();
        let t = title();
        if title_gen_done() || generating_title() {
            return;
        }
        if c.len() <= 50 || !is_auto_title(&t) {
            return;
        }
        generating_title.set(true);
        let preview: String = c.chars().take(1500).collect();
        spawn(async move {
            let lang = (app.current_lang)();
            if let Ok(ai) = crate::services::llm::LlmClient::from_db(&db()) {
                if let Ok(new_title) = ai.generate_title(&preview, &lang).await
                {
                    title.set(new_title);
                }
            }
            generating_title.set(false);
            title_gen_done.set(true);
        });
    });

    use_effect(move || {
        if pending_audio().is_some() && local_note_id().is_empty() {
            let t = title();
            let c = content();
            let new = NewTextNote {
                title: if t.is_empty() { None } else { Some(t.clone()) },
                content: c,
                tags: tags(),
            };
            if let Ok(created) = db().create_text_note(&new) {
                if let Some(ref fid) = (app.detail_folder_id)() {
                    let _ = db().add_note_to_folder(&created.id, fid);
                }
                if let Some((filename, dur)) = pending_audio() {
                    let _ = db().add_audio(&created.id, &filename, dur);
                }
                local_note_id.set(created.id.clone());
                app.current_note_id.set(Some(created.id));
                pending_audio.set(None);
                audios_version.set(audios_version() + 1);
                app.notes_version.set((app.notes_version)() + 1);
            }
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
        let folder = (app.detail_folder_id)();
        let lang_eff = (app.current_lang)();
        spawn(async move {
            import_in_progress.set(true);
            import_status.set(Some(t_args(
                &lang_eff,
                "note-import-in-progress",
                &[],
            )));
            let file = match import_file_content(&lang_eff).await {
                Ok(Some(f)) => f,
                Ok(None) => {
                    import_in_progress.set(false);
                    import_status.set(None);
                    return;
                }
                Err(e) => {
                    import_in_progress.set(false);
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
                            import_in_progress.set(false);
                            let msg = e.to_string();
                            import_status.set(Some(t_args(
                                &lang_eff,
                                "note-import-save-error",
                                &[("error", &msg)],
                            )));
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
                    import_in_progress.set(false);
                    import_status.set(None);
                }
                Err(e) => {
                    import_in_progress.set(false);
                    let msg = e.to_string();
                    import_status.set(Some(t_args(
                        &lang_eff,
                        "note-import-error",
                        &[("error", &msg)],
                    )));
                }
            }
        });
    });

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

    let observe_manager = manager.clone();
    use_effect(move || {
        let _ = (app.transcription_jobs)();
        let nid = local_note_id();
        if nid.is_empty() {
            return;
        }
        if let Some(text) = observe_manager.take_done(&nid) {
            let cur = content.peek().clone();
            if cur.is_empty() {
                content.set(text);
            } else {
                content.set(format!("{cur}\n{text}"));
            }
            app.transcription_jobs.set(observe_manager.snapshot());
        }
    });

    use_effect(move || {
        app.transcription_done_badge.set(0);
    });

    let recording_state = (app.recording_state)();
    let is_transcribing = recording_state == RecordingState::Transcribing;

    let modified_date = note
        .as_ref()
        .map(|n| format_relative_date(&n.modified_at, &lang))
        .unwrap_or_default();

    let created_date = note
        .as_ref()
        .map(|n| format_absolute_short(&n.created_at, &lang))
        .unwrap_or_default();

    let show_menu = (app.show_note_menu)();
    let title_placeholder = t(&lang, "note-title-placeholder");
    let content_placeholder = t(&lang, "note-content-placeholder");
    let transcribing_placeholder = t(&lang, "note-transcribing-placeholder");
    let created_on_text = if created_date.is_empty() {
        String::new()
    } else {
        t_args(&lang, "note-created-on", &[("date", &created_date)])
    };
    let is_importing = import_in_progress();

    rsx! {
        if show_menu {
            NoteMenu {
                note_id: local_note_id(),
                import_requested,
                deleted,
            }
        }
        div {
            class: "relative overflow-y-auto safe-pb-40 px-4 pt-3 lg:px-[max(1rem,calc((100%-48rem)/2))]",
            style: "height: calc(100% - var(--keyboard-inset, 0px));",
            if updated_from_peer() {
                button {
                    class: "w-full min-h-[44px] mb-2 px-3 py-2 rounded-xl bg-ios-orange/10 border border-ios-orange/30 text-ios-orange-dark text-xs font-medium text-left active:bg-ios-orange/25",
                    onclick: move |_| {
                        let id = local_note_id();
                        if let Some(fresh) = db().get_note(&id).ok().flatten() {
                            let fresh_title = fresh.title.clone().unwrap_or_default();
                            title.set(fresh_title.clone());
                            content.set(fresh.content.clone());
                            tags.set(fresh.tags.clone());
                            base_title.set(fresh_title);
                            base_content.set(fresh.content);
                            base_tags.set(fresh.tags);
                        }
                        updated_from_peer.set(false);
                    },
                    {t(&lang, "note-updated-from-peer")}
                }
            }
            div { class: "pt-2 pb-3",
                div { class: "inline-block relative",
                    span {
                        class: "text-xl font-semibold invisible whitespace-pre py-1.5 px-3 border border-transparent",
                        {
                            let t_disp = if title().is_empty() { title_placeholder.clone() } else { title() };
                            rsx! { "{t_disp}" }
                        }
                    }
                    input {
                        class: if generating_title() {
                            "absolute inset-0 text-xl font-semibold outline-none py-1.5 px-3 border border-stone-200/60 rounded-md text-stone-400 bg-white/25 animate-pulse"
                        } else {
                            "absolute inset-0 text-xl font-semibold outline-none py-1.5 px-3 border border-stone-200/60 rounded-md text-stone-900 bg-white/25 focus:border-ios-orange-dark/40 transition-colors duration-150"
                        },
                        placeholder: "{title_placeholder}",
                        value: "{title}",
                        oninput: move |evt| title.set(evt.value()),
                    }
                }
                if !modified_date.is_empty() {
                    div { class: "mt-2 px-1",
                        p { class: "text-xs text-stone-500", "{modified_date}" }
                        if !created_date.is_empty() {
                            p { class: "text-xs text-stone-400 mt-0.5", "{created_on_text}" }
                        }
                    }
                }
                ActiveReminders { local_note_id }
            }
            div { class: "border-t border-stone-100 pt-3 pb-2",
                TagsSection { tags, tag_input, tagging, content }
            }
            textarea {
                class: if is_transcribing {
                    "w-full min-h-[300px] border border-ios-orange/30 rounded-xl p-3 mt-3 text-sm resize-none font-sans outline-none text-stone-900"
                } else {
                    "w-full min-h-[300px] border border-stone-200 rounded-xl p-3 mt-3 text-sm resize-none font-sans outline-none text-stone-900"
                },
                placeholder: if is_transcribing { transcribing_placeholder.as_str() } else { content_placeholder.as_str() },
                value: "{content}",
                oninput: move |evt| content.set(evt.value()),
            }
            div { class: "flex justify-end mt-1",
                button {
                    class: if note_copied() {
                        "flex items-center gap-1 text-xs text-ios-green transition-colors duration-150"
                    } else {
                        "flex items-center gap-1 text-xs text-stone-400 active:text-stone-600 hover:text-stone-600 transition-colors duration-150"
                    },
                    onclick: move |_| {
                        if note_copied() {
                            return;
                        }
                        let note_title = title();
                        let body = content();
                        let text = if note_title.trim().is_empty() {
                            body
                        } else {
                            format!("{note_title}\n\n{body}")
                        };
                        crate::ui::clipboard::copy_text(&text);
                        note_copied.set(true);
                        spawn(async move {
                            futures_timer::Delay::new(
                                std::time::Duration::from_millis(1500),
                            )
                            .await;
                            note_copied.set(false);
                        });
                    },
                    if note_copied() {
                        IconCheck { size: 12 }
                        {t(&lang, "chat-copied")}
                    } else {
                        IconCopy { size: 12 }
                        {t(&lang, "chat-copy")}
                    }
                }
            }
            ReminderSuggestions {
                local_note_id,
                title,
                content,
                tags,
                initial_content: initial_content.clone(),
            }
            AudioSection { local_note_id, audios_version }
            AttachmentSection { attachments, confirm_delete_att }
            if let Some(ref status) = import_status() {
                div { class: if is_importing {
                        "flex items-center gap-2 px-3 py-2 mt-2 bg-ios-orange/10 rounded-lg"
                    } else {
                        "flex items-center gap-2 px-3 py-2 mt-2 bg-ios-red/10 rounded-lg"
                    },
                    if is_importing {
                        span { class: "inline-block w-3 h-3 border-2 border-ios-orange border-t-transparent rounded-full animate-spin" }
                    }
                    span { class: "text-xs text-stone-600", "{status}" }
                }
            }
            AudioJobBanner { local_note_id }
            if let RecordingState::Error(ref e) = recording_state {
                {
                    let msg = e.clone();
                    let recording_error = t_args(&lang, "note-recording-error", &[("message", &msg)]);
                    rsx! {
                        p { class: "text-xs text-stone-400 text-center mt-3",
                            "{recording_error}"
                        }
                    }
                }
            }
        }
        RecordingBar { pending_audio }
    }
}
