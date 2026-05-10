mod chat;
mod chat_input;
mod fab;
mod folder_picker;
pub mod icons;
mod note_card;
mod note_detail;
mod note_list;
mod recording_bar;
mod sidebar;
mod state;
mod top_bar;

pub use state::{AppState, View};

use crate::db::Database;
use crate::services::audio::{AudioRecorder, RecordingState};
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

use chat::ChatView;
use fab::FloatingActionButton;
use note_detail::NoteDetail;
use note_list::NotesList;
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
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        div { class: "h-screen w-full overflow-hidden font-sans bg-gray-100",
            SidebarOverlay {}
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
                    if matches!((app.view)(), View::Chat) {
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
                }
                if (app.view)() == View::NotesList {
                    FloatingActionButton {}
                }
            }
        }
    }
}
