use crate::application::i18n::{t, t_args};
use crate::application::note_persistence::create_note;
use crate::application::transcription_manager::TranscriptionManager;
use crate::domain::{generate_auto_title, Attachment};
use crate::infrastructure::audio::RecordingState;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::md_to_html;
use crate::ui::icons::{IconCheck, IconCopy, IconPencil};
use crate::ui::notes::attachments::AttachmentSection;
use crate::ui::notes::audio_section::{AudioJobBanner, AudioSection};
use crate::ui::notes::dates::{format_absolute_short, format_relative_date};
use crate::ui::notes::menu::NoteMenu;
use crate::ui::notes::reminders::{ActiveReminders, ReminderSuggestions};
use crate::ui::notes::tags::TagsSection;
use crate::ui::recording::RecordingBar;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

mod hooks;
use hooks::{
    use_audio_import, use_auto_title, use_document_import, use_peer_merge,
    use_save_on_drop, use_transcription_sink,
};

#[component]
pub fn NoteDetail() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let engine: Signal<Arc<crate::infrastructure::sync::engine::SyncEngine>> =
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
    let mut editing = use_signal(|| is_new);
    let mut tags: Signal<Vec<String>> = use_signal(|| initial_tags.clone());
    let mut base_title = use_signal(|| initial_title.clone());
    let mut base_content = use_signal(|| initial_content.clone());
    let mut base_tags: Signal<Vec<String>> =
        use_signal(|| initial_tags.clone());
    let mut updated_from_peer = use_signal(|| false);
    let tag_input = use_signal(String::new);
    let tagging = use_signal(|| false);
    let deleted = use_signal(|| false);
    let generating_title = use_signal(|| false);
    let title_gen_done = use_signal(|| false);
    let confirm_delete_att: Signal<Option<String>> = use_signal(|| None);
    let mut local_note_id = use_signal(|| note_id.clone());
    let import_requested = use_signal(|| false);
    let import_status: Signal<Option<String>> = use_signal(|| None);
    let import_in_progress = use_signal(|| false);
    let mut note_copied = use_signal(|| false);
    let mut pending_audio: Signal<Option<(String, f64)>> = use_signal(|| None);
    let mut audios_version = use_signal(|| 0u32);

    use_effect(move || {
        if !deleted() {
            app.current_note_id.set(Some(local_note_id()));
        }
    });

    use_peer_merge(
        app,
        db,
        local_note_id,
        deleted,
        title,
        content,
        tags,
        base_title,
        base_content,
        base_tags,
        updated_from_peer,
    );

    use_effect(move || {
        if deleted() {
            app.sliding_out.set(true);
            // spawn_forever: NoteDetail can unmount mid-delay (e.g. a sync-driven
            // rerender); a cancelled scope task would leave sliding_out stuck true.
            dioxus::core::spawn_forever(async move {
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

    use_save_on_drop(
        app,
        db,
        engine,
        title,
        content,
        tags,
        local_note_id,
        deleted,
        pending_audio,
        base_title,
        base_content,
        base_tags,
        initial_folder_id.clone(),
    );

    use_transcription_sink(
        app,
        db,
        engine,
        manager.clone(),
        content,
        base_content,
        local_note_id,
        audios_version,
    );

    use_auto_title(app, db, content, title, generating_title, title_gen_done);

    // Recording stopped before the note existed: `pending_audio` created it
    // here, so this is where its clip's transcription job gets queued.
    let deferred_manager = manager.clone();
    use_effect(move || {
        let Some((filename, _)) = pending_audio() else {
            return;
        };
        if !local_note_id().is_empty() {
            return;
        }
        let t = title();
        let c = content();
        let folder = (app.detail_folder_id)();
        let Some((created, audio_id)) = create_note(
            &db(),
            &t,
            &c,
            tags(),
            folder.as_deref(),
            pending_audio(),
        ) else {
            return;
        };
        local_note_id.set(created.id.clone());
        app.current_note_id.set(Some(created.id.clone()));
        pending_audio.set(None);
        audios_version.set(audios_version() + 1);
        app.notes_version.set((app.notes_version)() + 1);
        if let Some(aid) = audio_id {
            // The row stores the bare filename; the job needs a real path.
            let path =
                crate::infrastructure::audio::resolve_audio_path(&filename);
            deferred_manager.enqueue(
                created.id,
                std::path::PathBuf::from(path),
                Some(aid),
            );
        }
    });

    use_document_import(
        app,
        db,
        title,
        content,
        tags,
        local_note_id,
        import_requested,
        import_in_progress,
        import_status,
    );

    use_audio_import(
        app,
        db,
        manager.clone(),
        title,
        content,
        tags,
        local_note_id,
    );

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

    let note_sources_json = note.as_ref().and_then(|n| n.sources_json.clone());
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
                div { class: "relative w-full",
                    span {
                        class: "block w-full text-xl font-semibold invisible whitespace-pre-wrap break-words py-1.5 px-3 border border-transparent",
                        {
                            let t_disp = if title().is_empty() { title_placeholder.clone() } else { title() };
                            rsx! { "{t_disp} " }
                        }
                    }
                    textarea {
                        class: if generating_title() {
                            "absolute inset-0 w-full h-full resize-none overflow-hidden whitespace-pre-wrap break-words text-xl font-semibold outline-none py-1.5 px-3 border border-stone-200/60 rounded-lg text-stone-400 bg-white/25 animate-pulse"
                        } else {
                            "absolute inset-0 w-full h-full resize-none overflow-hidden whitespace-pre-wrap break-words text-xl font-semibold outline-none py-1.5 px-3 border border-stone-200/60 rounded-lg text-stone-900 bg-white/25 focus:border-ios-orange-dark/40 transition-colors duration-150"
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
            if editing() || is_transcribing {
                textarea {
                    class: if is_transcribing {
                        "w-full min-h-[300px] border border-ios-orange/30 rounded-xl p-3 mt-3 text-sm resize-none font-sans outline-none text-stone-900"
                    } else {
                        "w-full min-h-[300px] border border-stone-200 rounded-xl p-3 mt-3 text-sm resize-none font-sans outline-none text-stone-900 focus:border-stone-300 transition-colors duration-150"
                    },
                    placeholder: if is_transcribing { transcribing_placeholder.as_str() } else { content_placeholder.as_str() },
                    value: "{content}",
                    oninput: move |evt| content.set(evt.value()),
                }
            } else {
                div {
                    class: "w-full min-h-[300px] p-3 mt-3 text-sm prose prose-sm max-w-none text-stone-900 break-words overflow-x-hidden [&_*]:[overflow-wrap:anywhere] [&_pre]:whitespace-pre-wrap [&_pre]:break-words",
                    onclick: move |_| editing.set(true),
                    if content().trim().is_empty() {
                        span { class: "text-stone-400", "{content_placeholder}" }
                    } else {
                        div { dangerous_inner_html: md_to_html(&content()) }
                    }
                }
            }
            div { class: "flex justify-end gap-3 mt-1",
                button {
                    class: "flex items-center gap-1 text-xs text-stone-400 active:text-stone-600 hover:text-stone-600 transition-colors duration-150",
                    onclick: move |_| editing.set(!editing()),
                    if editing() {
                        IconCheck { size: 12 }
                        {t(&lang, "note-preview")}
                    } else {
                        IconPencil { size: 12 }
                        {t(&lang, "note-edit")}
                    }
                }
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
                        {t(&lang, "common-copied")}
                    } else {
                        IconCopy { size: 12 }
                        {t(&lang, "common-copy")}
                    }
                }
            }
            crate::ui::chat::NoteWebSources {
                sources_json: note_sources_json.clone(),
            }
            ReminderSuggestions {
                local_note_id,
                title,
                content,
                tags,
                initial_content: initial_content.clone(),
            }
            crate::ui::notes::note_actions::NoteActions {
                local_note_id,
                title,
                content,
                tags,
            }
            AudioSection { local_note_id, audios_version }
            AttachmentSection { attachments, confirm_delete_att }
            crate::ui::notes::related::RelatedSection { local_note_id }
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
