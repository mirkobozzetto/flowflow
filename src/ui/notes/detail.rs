use crate::db::Database;
use crate::models::{
    generate_auto_title, is_auto_title, Attachment, NewAttachment, NewTextNote,
    NoteAudio, NoteReminder, ReminderIntent, UpdateNote,
};
use crate::services::audio::{self, RecordingState};
use crate::services::embed::{embed_attachment, embed_note};
use crate::services::i18n::{month_abbr, reminder_due_label, t, t_args};
use crate::services::transcription::SonioxClient;
use crate::ui::folder_picker::FolderPicker;
use crate::ui::icons::{IconArrowUpRight, IconBell, IconTrash, IconX};
use crate::ui::notes::attachments::AttachmentSection;
use crate::ui::notes::audio_player::AudioPlayer;
use crate::ui::notes::menu::{
    import_audio_file, import_file_content, NoteMenu,
};
use crate::ui::notes::tags::TagsSection;
use crate::ui::recording::RecordingBar;
use crate::ui::transcription_manager::{JobStatus, TranscriptionManager};
use crate::ui::{AppState, View};
use chrono::{Datelike, Local, NaiveDateTime, NaiveTime, Timelike, Utc};
use dioxus::prelude::*;
use std::sync::Arc;
use std::time::Duration;

fn format_relative_date(iso: &str, lang: &str) -> String {
    let parsed = NaiveDateTime::parse_from_str(
        &iso.replace('T', " ").replace('Z', ""),
        "%Y-%m-%d %H:%M:%S%.f",
    );
    let dt = match parsed {
        Ok(d) => d,
        Err(_) => return iso.to_string(),
    };
    let now = Utc::now().naive_utc();
    let diff = now.signed_duration_since(dt);
    let secs = diff.num_seconds();

    if secs < 60 {
        return t(lang, "date-just-now");
    }
    if secs < 3600 {
        let mins = (secs / 60).to_string();
        return t_args(lang, "date-mins-ago", &[("mins", &mins)]);
    }
    if secs < 86400 {
        let hours = (secs / 3600).to_string();
        return t_args(lang, "date-hours-ago", &[("hours", &hours)]);
    }

    let today = now.date();
    let note_date = dt.date();
    if today.pred_opt() == Some(note_date) {
        let time = if lang == "fr" {
            dt.format("%H:%M").to_string()
        } else {
            dt.format("%-I:%M %p").to_string()
        };
        return t_args(lang, "date-yesterday", &[("time", &time)]);
    }

    let m = month_abbr(lang, note_date.month());
    let d = note_date.day();

    if note_date.year() == today.year() {
        format!("{d} {m}")
    } else {
        format!("{d} {m} {}", note_date.year())
    }
}

fn format_absolute_short(iso: &str, lang: &str) -> String {
    let parsed = NaiveDateTime::parse_from_str(
        &iso.replace('T', " ").replace('Z', ""),
        "%Y-%m-%d %H:%M:%S%.f",
    );
    let dt = match parsed {
        Ok(d) => d,
        Err(_) => return iso.to_string(),
    };
    let d = dt.date();
    let now = Utc::now().naive_utc().date();
    let m = month_abbr(lang, d.month());
    if d.year() == now.year() {
        let time = if lang == "fr" {
            dt.format("%H:%M").to_string()
        } else {
            dt.format("%-I:%M %p").to_string()
        };
        format!("{} {}, {}", d.day(), m, time)
    } else {
        format!("{} {} {}", d.day(), m, d.year())
    }
}

#[component]
pub fn NoteDetail() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
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
    let tags: Signal<Vec<String>> = use_signal(|| initial_tags.clone());
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
    let mut pending_audio: Signal<Option<(String, f64)>> = use_signal(|| None);
    let mut audios_version = use_signal(|| 0u32);
    let mut detected_reminders: Signal<Vec<ReminderIntent>> =
        use_signal(Vec::new);
    let mut reminders_checked = use_signal(String::new);
    let mut detecting_reminders = use_signal(|| false);
    let mut reminder_feedback: Signal<Option<String>> = use_signal(|| None);
    let mut confirm_delete_reminder: Signal<Option<String>> =
        use_signal(|| None);

    use_effect(move || {
        if !deleted() {
            app.current_note_id.set(Some(local_note_id()));
        }
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

    let audios: Vec<NoteAudio> = {
        let id = local_note_id();
        let _ = audios_version();
        let _ = (app.notes_version)();
        if id.is_empty() {
            Vec::new()
        } else {
            db().list_audios(&id).unwrap_or_default()
        }
    };
    let mut audios_expanded = use_signal(|| false);
    let mut transcribing_audio_id: Signal<Option<String>> = use_signal(|| None);

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

    let detail_reminders: Vec<NoteReminder> = {
        let id = local_note_id();
        let _ = (app.notes_version)();
        if id.is_empty() {
            Vec::new()
        } else {
            db().reminders_for_note(&id)
                .into_iter()
                .filter(|r| r.state == "active")
                .collect()
        }
    };

    use_drop({
        let orig_title = initial_title.clone();
        let orig_content = initial_content.clone();
        let orig_tags = initial_tags.clone();
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
            let title_changed = t != orig_title;
            let content_changed = c != orig_content;
            let tags_changed = tags() != orig_tags;
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
            if let Ok(ai) = crate::services::llm::LlmClient::from_env() {
                if let Ok(new_title) = ai.generate_title(&preview, &lang).await
                {
                    title.set(new_title);
                }
            }
            generating_title.set(false);
            title_gen_done.set(true);
        });
    });

    let reminder_initial_content = initial_content.clone();
    use_effect(move || {
        let c = content();
        if c.trim().chars().count() < 8 {
            if !detected_reminders.peek().is_empty() {
                detected_reminders.set(Vec::new());
            }
            return;
        }
        if *reminders_checked.peek() == c {
            return;
        }
        let nid = local_note_id();
        if c == reminder_initial_content
            && !nid.is_empty()
            && db().note_has_active_reminder(&nid)
        {
            reminders_checked.set(c);
            return;
        }
        spawn(async move {
            futures_timer::Delay::new(Duration::from_millis(1200)).await;
            if *content.peek() != c {
                return;
            }
            if *reminders_checked.peek() == c || *detecting_reminders.peek() {
                return;
            }
            detecting_reminders.set(true);
            match crate::services::llm::LlmClient::from_env() {
                Ok(ai) => {
                    if let Ok(intents) =
                        ai.extract_reminders(&c, Local::now()).await
                    {
                        let normalized: Vec<ReminderIntent> = intents
                            .into_iter()
                            .map(|mut it| {
                                if let Some(d) = it.resolved_date() {
                                    it.date =
                                        Some(d.format("%Y-%m-%d").to_string());
                                }
                                let tt = it.resolved_time();
                                it.time = Some(format!(
                                    "{:02}:{:02}",
                                    tt.hour(),
                                    tt.minute()
                                ));
                                it.time_end =
                                    it.resolved_time_end().map(|end| {
                                        format!(
                                            "{:02}:{:02}",
                                            end.hour(),
                                            end.minute()
                                        )
                                    });
                                it
                            })
                            .collect();
                        reminders_checked.set(c.clone());
                        detected_reminders.set(normalized);
                    }
                }
                Err(_) => {
                    reminders_checked.set(c.clone());
                }
            }
            detecting_reminders.set(false);
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

    let audio_job_status: Option<JobStatus> = {
        let jobs = (app.transcription_jobs)();
        let nid = local_note_id();
        if nid.is_empty() {
            None
        } else {
            jobs.get(&nid)
                .and_then(|q| q.front())
                .map(|j| j.status.clone())
        }
    };

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
    let transcribing_short = t(&lang, "note-transcribing-short");
    let transcribe_action = t(&lang, "note-transcribe-action");
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
            class: "relative overflow-y-auto pb-40 px-4 pt-3",
            style: "height: calc(100% - var(--keyboard-inset, 0px));",
            if (app.show_folder_picker)() {
                FolderPicker { selected: app.detail_folder_id }
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
                        p { class: "text-xs text-stone-400", "{modified_date}" }
                        if !created_date.is_empty() {
                            p { class: "text-[10px] text-stone-300 mt-0.5", "{created_on_text}" }
                        }
                    }
                }
                if !detail_reminders.is_empty() {
                    div { class: "mt-2 px-1 flex flex-col gap-1.5",
                        for r in detail_reminders.iter() {
                            {
                                let due = reminder_due_label(&lang, r);
                                let del_label = t(&lang, "reminder-delete");
                                let cancel_label = t(&lang, "reminder-cancel");
                                let row_id = r.id.clone();
                                let event_id_open = r.reminder_id.clone();
                                let event_id_del = r.reminder_id.clone();
                                let confirming = confirm_delete_reminder() == Some(row_id.clone());
                                rsx! {
                                    div { class: "flex items-center gap-1.5",
                                        button {
                                            class: "flex-1 flex items-center gap-2 min-h-[44px] px-3 py-2 rounded-xl bg-ios-orange/10 text-ios-orange-dark text-xs font-medium active:bg-ios-orange/25",
                                            onclick: move |_| {
                                                #[cfg(target_os = "ios")]
                                                {
                                                    let event_id_open = event_id_open.clone();
                                                    spawn(async move {
                                                        crate::platform::ios::reminders::present_event(event_id_open).await;
                                                    });
                                                }
                                                #[cfg(not(target_os = "ios"))]
                                                let _ = &event_id_open;
                                            },
                                            IconBell { size: 15 }
                                            span { class: "flex-1 text-left", "{due}" }
                                            IconArrowUpRight { size: 14 }
                                        }
                                        if confirming {
                                            button {
                                                class: "shrink-0 min-h-[44px] px-3 rounded-xl text-xs font-medium text-white bg-ios-red active:opacity-70",
                                                onclick: {
                                                    let row_id = row_id.clone();
                                                    let event_id_del = event_id_del.clone();
                                                    move |_| {
                                                        confirm_delete_reminder.set(None);
                                                        let _ = db().delete_note_reminder(&row_id);
                                                        app.notes_version.set((app.notes_version)() + 1);
                                                        #[cfg(target_os = "ios")]
                                                        {
                                                            let event_id_del = event_id_del.clone();
                                                            spawn(async move {
                                                                if let Err(e) = crate::platform::ios::reminders::remove_event(event_id_del).await {
                                                                    eprintln!("[reminder] remove_event failed: {e}");
                                                                }
                                                            });
                                                        }
                                                        #[cfg(not(target_os = "ios"))]
                                                        let _ = &event_id_del;
                                                    }
                                                },
                                                "{del_label}"
                                            }
                                            button {
                                                class: "shrink-0 min-h-[44px] px-2 text-xs font-medium text-stone-500 active:opacity-70",
                                                onclick: move |_| confirm_delete_reminder.set(None),
                                                "{cancel_label}"
                                            }
                                        } else {
                                            button {
                                                class: "shrink-0 min-h-[44px] p-2 text-stone-400 active:text-ios-red",
                                                onclick: {
                                                    let row_id = row_id.clone();
                                                    move |_| confirm_delete_reminder.set(Some(row_id.clone()))
                                                },
                                                IconTrash { size: 16 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
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
            {
                let detected = detected_reminders();
                let pending_idx: Vec<usize> = detected
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| {
                        let nid = local_note_id();
                        nid.is_empty()
                            || !db().reminder_exists_by_intent_hash(
                                &nid,
                                &i.intent_hash(),
                            )
                    })
                    .map(|(idx, _)| idx)
                    .collect();
                let title_label = t(&lang, "reminder-detected-title");
                let confirm_label = t(&lang, "reminder-confirm");
                let range_add_label = t(&lang, "reminder-range-add");
                let range_to_label = t(&lang, "reminder-range-to");
                let lang_badge = lang.clone();
                rsx! {
                    if !pending_idx.is_empty() {
                            div { class: "mt-3 p-3 bg-ios-orange/10 border border-ios-orange/30 rounded-xl",
                            div { class: "flex items-center justify-between mb-2",
                                span { class: "text-xs font-semibold text-ios-orange-dark", "{title_label}" }
                                button {
                                    class: "text-stone-400 active:opacity-70",
                                    onclick: move |_| {
                                        reminders_checked.set(content.peek().clone());
                                        detected_reminders.set(Vec::new());
                                    },
                                    IconX { size: 14 }
                                }
                            }
                            div { class: "space-y-2",
                                for idx in pending_idx {
                                    {
                                        let intent = detected[idx].clone();
                                        let action = intent.action.clone();
                                        let date_val = intent.date.clone().unwrap_or_default();
                                        let time_val = intent.time.clone().unwrap_or_default();
                                        let end_val = intent.time_end.clone().unwrap_or_default();
                                        let has_end = intent.time_end.is_some();
                                        let default_end = {
                                            use chrono::Duration as ChronoDuration;
                                            NaiveTime::parse_from_str(&time_val, "%H:%M")
                                                .ok()
                                                .map(|tm| tm + ChronoDuration::hours(1))
                                                .map(|tm| format!("{:02}:{:02}", tm.hour(), tm.minute()))
                                                .unwrap_or_else(|| "10:00".to_string())
                                        };
                                        let is_rec = intent.recurrence.is_some();
                                        let lang_cb = lang_badge.clone();
                                        let confirm = confirm_label.clone();
                                        let range_add = range_add_label.clone();
                                        let range_to = range_to_label.clone();
                                        rsx! {
                                            div { class: "bg-white/40 rounded-lg p-2.5",
                                                p { class: "text-sm text-stone-800 mb-1.5", "{action}" }
                                                div { class: "flex items-center gap-2 flex-wrap",
                                                    input {
                                                        r#type: "date",
                                                        class: "text-xs text-stone-700 bg-white border border-stone-200 rounded-md px-2 py-1 outline-none",
                                                        value: "{date_val}",
                                                        oninput: move |e| {
                                                            detected_reminders.write()[idx].date = Some(e.value());
                                                        },
                                                    }
                                                    input {
                                                        r#type: "time",
                                                        class: "text-xs text-stone-700 bg-white border border-stone-200 rounded-md px-2 py-1 outline-none",
                                                        value: "{time_val}",
                                                        oninput: move |e| {
                                                            detected_reminders.write()[idx].time = Some(e.value());
                                                        },
                                                    }
                                                    if has_end {
                                                        span { class: "text-xs text-stone-500", "{range_to}" }
                                                        input {
                                                            r#type: "time",
                                                            class: "text-xs text-stone-700 bg-white border border-stone-200 rounded-md px-2 py-1 outline-none",
                                                            value: "{end_val}",
                                                            oninput: move |e| {
                                                                detected_reminders.write()[idx].time_end = Some(e.value());
                                                            },
                                                        }
                                                        button {
                                                            class: "text-stone-400 active:opacity-70",
                                                            onclick: move |_| {
                                                                detected_reminders.write()[idx].time_end = None;
                                                            },
                                                            IconX { size: 12 }
                                                        }
                                                    } else {
                                                        button {
                                                            class: "text-xs font-medium text-ios-orange-dark active:opacity-70",
                                                            onclick: move |_| {
                                                                detected_reminders.write()[idx].time_end = Some(default_end.clone());
                                                            },
                                                            "{range_add}"
                                                        }
                                                    }
                                                    if is_rec {
                                                        span { class: "text-xs text-ios-orange-dark", "↻" }
                                                    }
                                                    button {
                                                    class: "shrink-0 self-center text-xs font-medium text-white bg-ios-orange-dark px-3 py-1.5 rounded-full active:opacity-70",
                                                    onclick: move |_| {
                                                        let intent = detected_reminders.peek()[idx].clone();
                                                        let lang2 = lang_cb.clone();
                                                        let database = db();
                                                        spawn(async move {
                                                            reminder_feedback.set(Some(t(&lang2, "reminder-creating")));
                                                            let nid = {
                                                                let cur = local_note_id();
                                                                if cur.is_empty() {
                                                                    let tt = title();
                                                                    let new = NewTextNote {
                                                                        title: if tt.is_empty() { None } else { Some(tt) },
                                                                        content: content(),
                                                                        tags: tags(),
                                                                    };
                                                                    match database.create_text_note(&new) {
                                                                        Ok(created) => {
                                                                            if let Some(ref fid) = (app.detail_folder_id)() {
                                                                                let _ = database.add_note_to_folder(&created.id, fid);
                                                                            }
                                                                            local_note_id.set(created.id.clone());
                                                                            app.current_note_id.set(Some(created.id.clone()));
                                                                            app.notes_version.set((app.notes_version)() + 1);
                                                                            created.id
                                                                        }
                                                                        Err(e) => {
                                                                            reminder_feedback.set(Some(t_args(&lang2, "reminder-failed", &[("error", &e)])));
                                                                            return;
                                                                        }
                                                                    }
                                                                } else {
                                                                    cur
                                                                }
                                                            };
                                                            let res = crate::services::reminders::schedule(database, nid, intent.clone()).await;
                                                            use crate::services::reminders::ScheduleResult;
                                                            match res {
                                                                ScheduleResult::Created => {
                                                                    app.notes_version.set((app.notes_version)() + 1);
                                                                    reminder_feedback.set(Some(t(&lang2, "reminder-created")));
                                                                }
                                                                ScheduleResult::Duplicate => reminder_feedback.set(Some(t(&lang2, "reminder-duplicate"))),
                                                                ScheduleResult::AccessDenied => reminder_feedback.set(Some(t(&lang2, "reminder-denied"))),
                                                                ScheduleResult::Unsupported => reminder_feedback.set(Some("iOS only".to_string())),
                                                                ScheduleResult::Failed(e) => reminder_feedback.set(Some(t_args(&lang2, "reminder-failed", &[("error", &e)]))),
                                                            }
                                                        });
                                                    },
                                                    "{confirm}"
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
            }
            if let Some(msg) = reminder_feedback() {
                div {
                    class: "mt-2 px-3 py-2 bg-ios-orange/10 rounded-lg text-xs text-stone-600",
                    "{msg}"
                }
            }
            if !audios.is_empty() {
                {
                    let count = audios.len().to_string();
                    let audio_label = t_args(&lang, "note-audios-label", &[("count", &count)]);
                    rsx! {
                        div { class: "mt-3",
                            button {
                                class: "flex items-center gap-2 w-full py-2 text-left active:opacity-70 transition-opacity duration-150",
                                onclick: move |_| audios_expanded.set(!audios_expanded()),
                                span { class: "text-xs font-medium text-stone-500", "{audio_label}" }
                                span {
                                    class: if audios_expanded() {
                                        "inline-block w-1.5 h-1.5 border-r-[1.5px] border-b-[1.5px] border-stone-400 transition-transform duration-150 -rotate-[135deg]"
                                    } else {
                                        "inline-block w-1.5 h-1.5 border-r-[1.5px] border-b-[1.5px] border-stone-400 transition-transform duration-150 rotate-45"
                                    },
                                }
                            }
                            if audios_expanded() {
                                div { class: "space-y-2 pt-1",
                                    for audio in audios.iter() {
                                        {
                                            let audio_id = audio.id.clone();
                                            let audio_id_tr = audio.id.clone();
                                            let file_path = audio.file_path.clone();
                                            let file_path_tr = audio.file_path.clone();
                                            let resolved = audio::resolve_audio_path(&file_path);
                                            let date = format_relative_date(&audio.created_at, &lang);
                                            let transcription = audio.transcription.clone();
                                            let is_transcribing_this = transcribing_audio_id() == Some(audio.id.clone());
                                            rsx! {
                                                div { class: "space-y-1",
                                                    p { class: "text-[10px] text-stone-400 px-1", "{date}" }
                                                    AudioPlayer {
                                                        audio_path: resolved,
                                                        duration_secs: audio.duration_secs,
                                                        on_delete: move |_| {
                                                            let fp = file_path.clone();
                                                            let _ = std::fs::remove_file(audio::resolve_audio_path(&fp));
                                                            let _ = db().delete_audio(&audio_id);
                                                            audios_version.set(audios_version() + 1);
                                                            app.notes_version.set((app.notes_version)() + 1);
                                                        },
                                                    }
                                                    if let Some(ref text) = transcription {
                                                        p { class: "text-xs text-stone-600 italic px-1 pb-1", "{text}" }
                                                    } else if is_transcribing_this {
                                                        p {
                                                            class: "text-xs text-stone-400 px-1 pb-1",
                                                            style: "animation: pulseSoft 1.5s ease-in-out infinite;",
                                                            "{transcribing_short}"
                                                        }
                                                    } else {
                                                        button {
                                                            class: "text-xs text-ios-orange-dark px-1 pb-1 active:opacity-70",
                                                            onclick: move |_| {
                                                                let aid = audio_id_tr.clone();
                                                                let fp = file_path_tr.clone();
                                                                let path = audio::resolve_audio_path(&fp);
                                                                transcribing_audio_id.set(Some(aid.clone()));
                                                                spawn(async move {
                                                                    let result = async {
                                                                        let client = SonioxClient::from_env()?;
                                                                        client.transcribe(std::path::Path::new(&path), None).await
                                                                    }.await;
                                                                    match result {
                                                                        Ok(text) => {
                                                                            let _ = db().set_audio_transcription(&aid, &text);
                                                                        }
                                                                        Err(e) => {
                                                                            eprintln!("[transcribe] error: {e}");
                                                                        }
                                                                    }
                                                                    transcribing_audio_id.set(None);
                                                                    audios_version.set(audios_version() + 1);
                                                                });
                                                            },
                                                            "{transcribe_action}"
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
                }
            }
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
            if let Some(ref js) = audio_job_status {
                match js {
                    JobStatus::Failed(reason) => {
                        let failed_label = t(&lang, "audio-import-failed");
                        let retry_label = t(&lang, "audio-import-retry");
                        let msg = format!("{failed_label}: {reason}");
                        let retry_manager = manager.clone();
                        let dismiss_manager = manager.clone();
                        rsx! {
                            div { class: "flex items-center gap-2 px-3 py-2 mt-2 bg-ios-red/10 rounded-lg",
                                span { class: "text-xs text-stone-600 flex-1", "{msg}" }
                                button {
                                    class: "text-xs text-ios-orange-dark font-medium active:opacity-70",
                                    onclick: move |_| {
                                        let nid = local_note_id();
                                        retry_manager.retry(&nid);
                                    },
                                    "{retry_label}"
                                }
                                button {
                                    class: "text-stone-400 active:opacity-70",
                                    onclick: move |_| {
                                        let nid = local_note_id();
                                        dismiss_manager.dismiss(&nid);
                                    },
                                    IconX { size: 14 }
                                }
                            }
                        }
                    }
                    JobStatus::Done(_) => rsx! {},
                    other => {
                        let transcribing_label = t(&lang, "audio-transcribing");
                        let label = match other {
                            JobStatus::Polling { elapsed_s } => {
                                format!("{transcribing_label} · {}:{:02}", elapsed_s / 60, elapsed_s % 60)
                            }
                            _ => transcribing_label,
                        };
                        rsx! {
                            div { class: "flex items-center gap-2 px-3 py-2 mt-2 bg-ios-orange/10 rounded-lg",
                                span { class: "inline-block w-3 h-3 border-2 border-ios-orange border-t-transparent rounded-full animate-spin" }
                                span { class: "text-xs text-stone-600", "{label}" }
                            }
                        }
                    }
                }
            }
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
