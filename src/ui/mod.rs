mod attachment_modal;
mod chat;
mod chat_input;
mod consent;
mod fab;
pub(crate) mod folder_picker;
pub mod icons;
mod note_card;
mod note_list;
mod notes;
mod recording;
mod restore_lock;
mod settings;
mod sidebar;
mod state;
mod sync;
mod top_bar;
pub mod transcription_manager;

pub use state::{AppState, SettingsSection, SidebarTab, View};

use crate::db::Database;
use crate::services::audio::{AudioRecorder, RecordingState};
use crate::services::sync::engine::SyncEngine;
use crate::services::sync::reconcile::run_boot_reconcile;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use transcription_manager::{
    append_transcription_to_note, JobStatus, TranscriptionManager,
};

use attachment_modal::AttachmentModal;
use chat::ChatView;
use consent::ConsentScreen;
use fab::FloatingActionButton;
use note_list::NotesList;
use notes::NoteDetail;
use restore_lock::RestoreLockScreen;
use settings::{SettingsSectionView, SettingsView};
use sidebar::SidebarOverlay;
use sync::SyncView;
use top_bar::TopBar;

#[component]
pub fn App() -> Element {
    let _db = use_context_provider(|| {
        let db = Arc::new(Database::open().expect("Failed to open database"));
        if crate::services::backup::restore_recovery_window_active() {
            eprintln!(
                "[backup] orphan audio cleanup skipped (recovery window)"
            );
        } else {
            db.cleanup_orphan_audio(&crate::services::audio::output_dir());
        }
        crate::services::backup::finalize_restore_bak();
        run_boot_reconcile();
        #[cfg(target_os = "ios")]
        {
            crate::platform::ios::sync_ffi::observe_background_checkpoint();
            crate::platform::ios::sync_ffi::observe_restore_foreground();
        }
        Signal::new(db)
    });

    let mut restore_locked =
        use_signal(crate::services::backup::restore_lock_active);
    let mut index_rebuilding = use_signal(|| false);

    let _engine: Signal<Arc<SyncEngine>> =
        use_context_provider(|| Signal::new(SyncEngine::start(_db())));

    let _recorder: Signal<Arc<Mutex<AudioRecorder>>> =
        use_context_provider(|| {
            Signal::new(Arc::new(Mutex::new(AudioRecorder::new())))
        });

    let manager = use_context_provider(|| {
        let m = TranscriptionManager::new(_db());
        m.resume_pending();
        m
    });

    if option_env!("FLOWFLOW_RESET_CONSENT") == Some("1") {
        let _ = _db().set_setting("ai_consent", "");
    }
    let consent_value = _db().get_setting("ai_consent").map(|v| v == "true");

    let initial_lang = _db()
        .get_setting(crate::db::settings_repo::LANGUAGE_KEY)
        .unwrap_or_else(crate::platform::detect_system_language);

    let app = use_context_provider(|| AppState {
        view: Signal::new(View::NotesList),
        sidebar_open: Signal::new(false),
        selected_folder_id: Signal::new(None),
        recording_state: Signal::new(RecordingState::Idle),
        folders_version: Signal::new(0),
        sliding_out: Signal::new(false),
        audio_levels: Signal::new(vec![0.0; 12]),
        notes_version: Signal::new(0),
        current_note_id: Signal::new(None),
        previous_view: Signal::new(None),
        search_query: Signal::new(String::new()),
        show_note_menu: Signal::new(false),
        attachments_version: Signal::new(0),
        attachment_modal: Signal::new(None),
        show_chat_menu: Signal::new(false),
        sidebar_tab: Signal::new(SidebarTab::Notes),
        show_folder_picker: Signal::new(false),
        chat_scope_folder_id: Signal::new(None),
        detail_folder_id: Signal::new(None),
        ai_consent: Signal::new(consent_value),
        current_lang: Signal::new(initial_lang),
        transcription_jobs: Signal::new(HashMap::new()),
        transcription_done_badge: Signal::new(0),
        audio_import_requested: Signal::new(false),
        sync_data_version: Signal::new(0),
    });

    use_future(move || {
        let manager = manager.clone();
        let db = _db();
        let mut app = app;
        async move {
            loop {
                let snap = manager.snapshot();
                if *app.transcription_jobs.peek() != snap {
                    app.transcription_jobs.set(snap.clone());
                }
                let current = (app.current_note_id)();
                let viewing_note =
                    matches!((app.view)(), View::NoteDetail { .. });
                for (note_id, q) in snap.iter() {
                    let is_done = matches!(
                        q.front().map(|j| &j.status),
                        Some(JobStatus::Done(_))
                    );
                    let is_open = viewing_note
                        && current.as_deref() == Some(note_id.as_str());
                    if is_done && !is_open {
                        if let Some(text) = manager.take_done(note_id) {
                            append_transcription_to_note(&db, note_id, &text);
                            app.notes_version.set((app.notes_version)() + 1);
                            app.transcription_done_badge
                                .set((app.transcription_done_badge)() + 1);
                            _engine.peek().schedule_debounced();
                        }
                    }
                }
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    700,
                ))
                .await;
            }
        }
    });

    use_future(move || {
        let mut app = app;
        async move {
            let mut last_seen = _engine.peek().data_version();
            loop {
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    400,
                ))
                .await;
                let lock_now = crate::services::backup::restore_lock_active();
                if *restore_locked.peek() != lock_now {
                    restore_locked.set(lock_now);
                }
                let rebuilding =
                    crate::services::backup::restore_recovery_window_active()
                        && crate::services::sync::reconcile::reconcile_running(
                        );
                if *index_rebuilding.peek() != rebuilding {
                    index_rebuilding.set(rebuilding);
                }
                let current_view = (app.view)();
                if sync::pending_pairing_uri_exists()
                    && current_view != View::SyncPairing
                {
                    app.previous_view.set(Some(current_view));
                    app.view.set(View::SyncPairing);
                }
                let current = _engine.peek().data_version();
                if current == last_seen {
                    continue;
                }
                last_seen = current;
                app.sync_data_version.set(current);
                app.notes_version.set((app.notes_version)() + 1);
                app.folders_version.set((app.folders_version)() + 1);
                app.attachments_version.set((app.attachments_version)() + 1);
            }
        }
    });

    use_effect(|| {
        dioxus::document::eval(
            r#"
            (function() {
                var cachedKeyboardH = 0;

                function applyOffset(offset) {
                    document.documentElement.style.setProperty('--keyboard-inset', offset + 'px');
                    var els = document.querySelectorAll('.keyboard-aware');
                    for (var i = 0; i < els.length; i++) {
                        els[i].style.bottom = offset + 'px';
                    }
                }

                function measureKeyboard() {
                    if (!window.visualViewport) return 0;
                    var vv = window.visualViewport;
                    return Math.max(0, window.innerHeight - vv.height - vv.offsetTop);
                }

                if (window.visualViewport) {
                    var handler = function() {
                        var offset = measureKeyboard();
                        if (offset > 50) cachedKeyboardH = offset;
                        applyOffset(offset);
                    };
                    window.visualViewport.addEventListener('resize', handler);
                    window.visualViewport.addEventListener('scroll', handler);
                }

                document.addEventListener('focusin', function(e) {
                    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
                        if (cachedKeyboardH > 0) {
                            applyOffset(cachedKeyboardH);
                        }
                        setTimeout(function() {
                            var h = measureKeyboard();
                            if (h > 50) {
                                cachedKeyboardH = h;
                                applyOffset(h);
                            }
                            e.target.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
                        }, 400);
                    }
                });

                document.addEventListener('focusout', function() {
                    applyOffset(0);
                    requestAnimationFrame(function() {
                        window.scrollTo(0, window.scrollY);
                    });
                });
            })();
            "#,
        );
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        if restore_locked() {
            RestoreLockScreen {}
        } else if (app.ai_consent)() != Some(true) {
            ConsentScreen {}
        } else {
            div { class: "h-screen w-full overflow-hidden font-sans bg-stone-100 lg:flex lg:flex-row",
                SidebarOverlay {}
                AttachmentModal {}
                div { class: "flex flex-col h-screen safe-pt lg:flex-1 lg:min-w-0",
                    TopBar {}
                    if index_rebuilding() {
                        div { class: "bg-ios-orange/10 border-b border-ios-orange/20 px-4 py-1.5",
                            p { class: "text-xs text-ios-orange text-center",
                                {crate::services::i18n::t(&(app.current_lang)(), "restore-banner-rebuilding")}
                            }
                        }
                    }
                    div { class: "flex-1 overflow-hidden relative",
                        {
                            let is_bg = !matches!((app.view)(), View::NotesList);
                            let is_note = matches!((app.view)(), View::NoteDetail { .. });
                            let sliding_back = (app.sliding_out)();
                            let shifted = is_bg && !sliding_back;
                            let shift_dir = if is_note { "30%" } else { "-30%" };
                            rsx! {
                                div {
                                    class: "absolute inset-0 overflow-y-auto px-4 py-3 safe-pb-20",
                                    class: if is_bg { "pointer-events-none" } else { "" },
                                    style: if shifted {
                                        format!("transform: translateX({shift_dir}); opacity: 0.5; transition: transform 0.15s ease, opacity 0.15s ease;")
                                    } else {
                                        "transform: translateX(0); opacity: 1; transition: transform 0.15s ease, opacity 0.15s ease;".to_string()
                                    },
                                    div { class: "w-full lg:max-w-3xl lg:mx-auto",
                                        NotesList {}
                                    }
                                }
                            }
                        }
                        if matches!((app.view)(), View::NoteDetail { .. }) {
                            div {
                                class: "absolute inset-0 flex flex-col min-h-0 bg-stone-100",
                                style: if (app.sliding_out)() {
                                    "animation: slideOutToLeft 0.15s ease-in forwards;"
                                } else {
                                    "animation: slideInFromLeft 0.15s ease-out;"
                                },
                                div { class: "w-full lg:max-w-3xl lg:mx-auto flex-1 flex flex-col min-h-0",
                                    NoteDetail {}
                                }
                            }
                        }
                        if matches!((app.view)(), View::Chat { .. }) {
                            div {
                                class: "absolute inset-0 flex flex-col min-h-0 bg-stone-100",
                                style: if (app.sliding_out)() {
                                    "animation: slideOutRight 0.15s ease-in forwards;"
                                } else {
                                    "animation: slideInRight 0.15s ease-out;"
                                },
                                div { class: "w-full lg:max-w-3xl lg:mx-auto flex-1 flex flex-col min-h-0",
                                    ChatView {}
                                }
                            }
                        }
                        {
                            let view_now = (app.view)();
                            let pairing_from_settings = matches!(view_now, View::SyncPairing)
                                && (app.previous_view)() == Some(View::Settings);
                            let settings_stack = matches!(view_now, View::Settings | View::SettingsSection(_))
                                || pairing_from_settings;
                            let in_section = settings_stack && !matches!(view_now, View::Settings);
                            let sliding_back = (app.sliding_out)();
                            if settings_stack {
                                rsx! {
                                    div {
                                        class: "absolute inset-0 flex flex-col min-h-0 px-4 safe-py-3 bg-stone-100 overflow-y-auto",
                                        class: if in_section && !sliding_back { "pointer-events-none" } else { "" },
                                        style: if sliding_back && !in_section {
                                            "animation: slideOutRight 0.15s ease-in forwards;"
                                        } else if in_section && !sliding_back {
                                            "animation: slideInRight 0.15s ease-out; transform: translateX(-30%); opacity: 0.5; transition: transform 0.15s ease, opacity 0.15s ease;"
                                        } else {
                                            "animation: slideInRight 0.15s ease-out; transform: translateX(0); opacity: 1; transition: transform 0.15s ease, opacity 0.15s ease;"
                                        },
                                        div { class: "w-full lg:max-w-2xl lg:mx-auto",
                                            SettingsView {}
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                        if matches!((app.view)(), View::SettingsSection(_)) {
                            div {
                                class: "absolute inset-0 flex flex-col min-h-0 px-4 safe-py-3 bg-stone-100 overflow-y-auto",
                                style: if (app.sliding_out)() {
                                    "animation: slideOutRight 0.15s ease-in forwards;"
                                } else {
                                    "animation: slideInRight 0.15s ease-out;"
                                },
                                div { class: "w-full lg:max-w-2xl lg:mx-auto",
                                    SettingsSectionView {}
                                }
                            }
                        }
                        if matches!((app.view)(), View::SyncPairing) {
                            div {
                                class: "absolute inset-0 flex flex-col min-h-0 px-4 safe-py-3 bg-stone-100 overflow-y-auto",
                                style: if (app.sliding_out)() {
                                    "animation: slideOutRight 0.15s ease-in forwards;"
                                } else {
                                    "animation: slideInRight 0.15s ease-out;"
                                },
                                div { class: "w-full lg:max-w-2xl lg:mx-auto",
                                    SyncView {}
                                }
                            }
                        }
                    }
                    if (app.view)() == View::NotesList {
                        FloatingActionButton {}
                    }
                }
            }
        }
    }
}
