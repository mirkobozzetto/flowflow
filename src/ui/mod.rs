mod layout;
mod notes;
mod recording;

use crate::db::Database;
use crate::services::audio::{self, AudioRecorder, RecordingState};
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

use layout::{FloatingActionButton, SidebarOverlay, TopBar};
use notes::{NoteDetail, NotesList};
use recording::RecordingView;

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    NotesList,
    NoteDetail { note_id: String },
    Recording,
}

#[derive(Clone)]
pub struct AppState {
    pub view: Signal<View>,
    pub sidebar_open: Signal<bool>,
    pub selected_folder_id: Signal<Option<String>>,
    pub recording_state: Signal<RecordingState>,
}

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
    });

    let has_mic = use_signal(audio::has_input_device);

    rsx! {
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        div { class: "h-screen w-full overflow-hidden font-sans bg-gray-100",
            SidebarOverlay {}
            div { class: "flex flex-col h-screen",
                TopBar {}
                div {
                    class: match (app.view)() {
                        View::NotesList => "flex-1 overflow-y-auto px-4 py-3 pb-20",
                        _ => "flex-1 overflow-hidden px-4 py-3",
                    },
                    match (app.view)() {
                        View::NotesList => rsx! { NotesList {} },
                        View::NoteDetail { .. } => rsx! { NoteDetail {} },
                        View::Recording => rsx! {
                            if has_mic() {
                                RecordingView {}
                            } else {
                                p { class: "text-sm text-gray-400 text-center pt-10",
                                    "Pas de micro détecté."
                                }
                            }
                        },
                    }
                }
                if (app.view)() == View::NotesList {
                    FloatingActionButton {}
                }
            }
        }
    }
}
