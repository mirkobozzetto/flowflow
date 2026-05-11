mod attachment_modal;
mod chat;
mod chat_input;
mod fab;
mod folder_picker;
pub mod icons;
mod note_card;
mod note_detail;
mod note_list;
mod recording_bar;
mod settings;
mod sidebar;
mod state;
mod top_bar;

pub use state::{AppState, View};

use crate::db::Database;
use crate::services::audio::{AudioRecorder, RecordingState};
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

use attachment_modal::AttachmentModal;
use chat::ChatView;
use fab::FloatingActionButton;
use note_detail::NoteDetail;
use note_list::NotesList;
use settings::SettingsView;
use sidebar::SidebarOverlay;
use top_bar::TopBar;

#[component]
pub fn App() -> Element {
    let _db = use_context_provider(|| {
        Signal::new(Arc::new(
            Database::open().expect("Failed to open database"),
        ))
    });

    let _recorder: Signal<Arc<Mutex<AudioRecorder>>> =
        use_context_provider(|| {
            Signal::new(Arc::new(Mutex::new(AudioRecorder::new())))
        });

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

        div { class: "h-screen w-full overflow-hidden font-sans bg-gray-100",
            SidebarOverlay {}
            AttachmentModal {}
            div { class: "flex flex-col h-screen",
                TopBar {}
                div { class: "flex-1 overflow-hidden relative",
                    div {
                        class: "absolute inset-0 overflow-y-auto px-4 py-3 pb-20",
                        class: if !matches!((app.view)(), View::NotesList) { "pointer-events-none" } else { "" },
                        NotesList {}
                    }
                    if matches!((app.view)(), View::NoteDetail { .. }) {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 px-4 py-3 bg-gray-100",
                            style: if (app.sliding_out)() {
                                "animation: slideOutRight 0.15s ease-in forwards;"
                            } else {
                                "animation: slideInRight 0.15s ease-out;"
                            },
                            NoteDetail {}
                        }
                    }
                    if matches!((app.view)(), View::Chat { .. }) {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 bg-gray-100",
                            style: if (app.sliding_out)() {
                                "animation: slideOutRight 0.15s ease-in forwards;"
                            } else {
                                "animation: slideInRight 0.15s ease-out;"
                            },
                            ChatView {}
                        }
                    }
                    if matches!((app.view)(), View::Settings) {
                        div {
                            class: "absolute inset-0 flex flex-col min-h-0 px-4 py-3 bg-gray-100",
                            style: if (app.sliding_out)() {
                                "animation: slideOutRight 0.15s ease-in forwards;"
                            } else {
                                "animation: slideInRight 0.15s ease-out;"
                            },
                            SettingsView {}
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
