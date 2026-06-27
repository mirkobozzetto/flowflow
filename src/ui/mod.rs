mod action_card;
mod app;
mod attachment_modal;
mod chat;
mod chat_input;
mod clipboard;
mod consent;
mod fab;
pub(crate) mod folder_picker;
pub mod icons;
mod keyboard;
pub(crate) mod kit;
mod note_card;
mod note_list;
mod notes;
mod recording;
mod restore_lock;
mod settings;
mod sidebar;
mod state;
mod sync;
mod thread;
mod top_bar;
pub mod transcription_manager;

pub use state::{AppState, SettingsSection, SidebarTab, View};

use crate::infrastructure::audio::AudioRecorder;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::engine::SyncEngine;
use crate::infrastructure::sync::reconcile::run_boot_reconcile;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};
use transcription_manager::TranscriptionManager;

use app::{
    use_history_tracker, use_picker_reset_on_view, use_sync_watcher,
    use_transcription_watcher,
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
use thread::ThreadDetail;
use top_bar::TopBar;

#[component]
pub fn App() -> Element {
    let _db = use_context_provider(|| {
        let db = Arc::new(Database::open().expect("Failed to open database"));
        if crate::application::backup::restore_recovery_window_active() {
            eprintln!(
                "[backup] orphan audio cleanup skipped (recovery window)"
            );
        } else {
            db.cleanup_orphan_audio(&crate::infrastructure::audio::output_dir());
        }
        crate::application::backup::finalize_restore_bak();
        run_boot_reconcile();
        #[cfg(target_os = "ios")]
        {
            crate::infrastructure::platform::ios::sync_ffi::observe_background_checkpoint();
            crate::infrastructure::platform::ios::sync_ffi::observe_restore_foreground();
        }
        Signal::new(db)
    });

    let restore_locked =
        use_signal(crate::application::backup::restore_lock_active);
    let index_rebuilding = use_signal(|| false);

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
        .get_setting(
            crate::infrastructure::persistence::settings_repo::LANGUAGE_KEY,
        )
        .unwrap_or_else(
            crate::infrastructure::platform::detect_system_language,
        );

    let app =
        use_context_provider(|| AppState::new(consent_value, initial_lang));

    // ponytail: temporary spike trigger for issue #43, delete after device validation
    #[cfg(debug_assertions)]
    use_future(move || {
        let db = _db();
        async move {
            let key = crate::application::web_search::exa_api_key(&db);
            let results = crate::application::web_search::exa_search(
                "latest rust async runtime news",
                &key,
            )
            .await;
            eprintln!(
                "[exa spike #43] {} results: {:#?}",
                results.len(),
                results
            );
        }
    });

    use_transcription_watcher(manager.clone(), _db, _engine, app);
    use_sync_watcher(_engine, app, restore_locked, index_rebuilding);
    use_picker_reset_on_view(app);
    use_history_tracker(app);

    #[cfg(target_os = "macos")]
    keyboard::use_macos_shortcuts(app);
    keyboard::use_keyboard_inset();

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
                                {crate::application::i18n::t(&(app.current_lang)(), "restore-banner-rebuilding")}
                            }
                        }
                    }
                    div { class: "flex-1 overflow-hidden relative",
                        {
                            let is_bg = !matches!((app.view)(), View::NotesList);
                            let is_note = matches!((app.view)(), View::NoteDetail { .. } | View::ThreadDetail { .. });
                            let sliding_back = (app.sliding_out)();
                            let shifted = is_bg && !sliding_back;
                            let shift_dir = if is_note { "30%" } else { "-30%" };
                            let instant = cfg!(target_os = "macos");
                            rsx! {
                                div {
                                    id: "notes-scroll",
                                    class: "absolute inset-0 overflow-y-auto px-4 py-3 safe-pb-20",
                                    class: if is_bg { "pointer-events-none" } else { "" },
                                    style: if instant && shifted {
                                        format!("transform: translateX({shift_dir}); opacity: 0.5;")
                                    } else if instant {
                                        "transform: translateX(0); opacity: 1;".to_string()
                                    } else if shifted {
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
                                style: if cfg!(target_os = "macos") {
                                    ""
                                } else if (app.sliding_out)() {
                                    "animation: slideOutToLeft 0.15s ease-in forwards;"
                                } else {
                                    "animation: slideInFromLeft 0.15s ease-out;"
                                },
                                div { class: "w-full flex-1 flex flex-col min-h-0",
                                    NoteDetail {}
                                }
                            }
                        }
                        if matches!((app.view)(), View::ThreadDetail { .. }) {
                            div {
                                class: "absolute inset-0 flex flex-col min-h-0 bg-stone-100",
                                style: if cfg!(target_os = "macos") {
                                    ""
                                } else if (app.sliding_out)() {
                                    "animation: slideOutToLeft 0.15s ease-in forwards;"
                                } else {
                                    "animation: slideInFromLeft 0.15s ease-out;"
                                },
                                div { class: "w-full flex-1 flex flex-col min-h-0",
                                    ThreadDetail {}
                                }
                            }
                        }
                        if matches!((app.view)(), View::Chat { .. }) {
                            div {
                                class: "absolute inset-0 flex flex-col min-h-0 bg-stone-100",
                                style: if cfg!(target_os = "macos") {
                                    ""
                                } else if (app.sliding_out)() {
                                    "animation: slideOutRight 0.15s ease-in forwards;"
                                } else {
                                    "animation: slideInRight 0.15s ease-out;"
                                },
                                div { class: "w-full flex-1 flex flex-col min-h-0",
                                    ChatView {}
                                }
                            }
                        }
                        if matches!(
                            (app.view)(),
                            View::Settings | View::SettingsSection(_)
                        ) || (matches!((app.view)(), View::SyncPairing)
                            && (app.previous_view)() == Some(View::Settings))
                        {
                            div {
                                class: "absolute inset-0 flex flex-col min-h-0 px-4 safe-py-3 bg-stone-100 overflow-y-auto",
                                class: if !matches!((app.view)(), View::Settings)
                                    && !(app.sliding_out)()
                                {
                                    "pointer-events-none"
                                } else {
                                    ""
                                },
                                style: {
                                    let in_section =
                                        !matches!((app.view)(), View::Settings);
                                    let sliding_back = (app.sliding_out)();
                                    if cfg!(target_os = "macos") {
                                        // Desktop: opaque, no depth dimming. The dim+shift
                                        // left a ghost settings panel on transitions.
                                        String::new()
                                    } else if sliding_back && !in_section {
                                        "animation: slideOutRight 0.15s ease-in forwards;".to_string()
                                    } else if sliding_back && in_section {
                                        // Going back from a section: un-shift the list to 0 in
                                        // parallel with the section sliding out, instead of waiting
                                        // for the view flip (which animated them in sequence). Keep
                                        // the slideInRight token so it never replays here.
                                        "animation: slideInRight 0.15s ease-out; transform: translateX(0); opacity: 1; transition: transform 0.15s ease, opacity 0.15s ease;".to_string()
                                    } else if in_section {
                                        "animation: slideInRight 0.15s ease-out; transform: translateX(-30%); opacity: 0.5; transition: transform 0.15s ease, opacity 0.15s ease;".to_string()
                                    } else {
                                        "animation: slideInRight 0.15s ease-out; transform: translateX(0); opacity: 1; transition: transform 0.15s ease, opacity 0.15s ease;".to_string()
                                    }
                                },
                                div { class: "w-full lg:max-w-2xl lg:mx-auto",
                                    SettingsView {}
                                }
                            }
                        }
                        if matches!((app.view)(), View::SettingsSection(_)) {
                            div {
                                class: "absolute inset-0 flex flex-col min-h-0 px-4 safe-py-3 bg-stone-100 overflow-y-auto",
                                style: if cfg!(target_os = "macos") {
                                    ""
                                } else if (app.sliding_out)() {
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
                                style: if cfg!(target_os = "macos") {
                                    ""
                                } else if (app.sliding_out)() {
                                    "animation: slideOutRight 0.15s ease-in forwards;"
                                } else {
                                    "animation: slideInRight 0.15s ease-out;"
                                },
                                div { class: "w-full lg:max-w-2xl lg:mx-auto",
                                    SyncView {}
                                }
                            }
                        }
                        if (app.show_folder_picker)() {
                            div {
                                class: "fixed inset-0 z-10",
                                onclick: move |_| {
                                    let mut app = app;
                                    app.show_folder_picker.set(false);
                                },
                            }
                            {
                                match (app.view)() {
                                    View::NotesList => rsx! {
                                        folder_picker::FolderPicker { selected: app.selected_folder_id, on_pick: move |_| {} }
                                    },
                                    View::NoteDetail { .. } => rsx! {
                                        folder_picker::FolderPicker { selected: app.detail_folder_id, on_pick: move |_| {} }
                                    },
                                    View::Chat { .. } => rsx! {
                                        folder_picker::ChatScopePicker {}
                                    },
                                    _ => rsx! {},
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
