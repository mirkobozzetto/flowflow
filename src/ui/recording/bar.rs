use crate::services::audio::{AudioRecorder, RecordingState};
use crate::ui::icons::*;
use crate::ui::recording::{start_recording, RecordingControls};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

#[component]
pub fn RecordingBar(pending_audio: Signal<Option<(String, f64)>>) -> Element {
    let app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let recording_state = (app.recording_state)();
    let is_idle = recording_state == RecordingState::Idle
        || matches!(recording_state, RecordingState::Error(_))
        || matches!(recording_state, RecordingState::Transcribed(_));

    rsx! {
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-2 bg-warm-white border-t border-stone-200 z-30 keyboard-aware",
            if is_idle {
                div { class: "flex items-center justify-center",
                    button {
                        class: "w-full flex items-center justify-center gap-2.5 h-12 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-sm font-medium",
                        onclick: move |_| start_recording(recorder, app),
                        IconMic { size: 22 }
                        span { "Dicter" }
                    }
                }
                if let RecordingState::Error(ref e) = recording_state {
                    p { class: "text-xs text-ios-red text-center mt-1", "{e}" }
                }
            } else {
                RecordingControls { pending_audio }
            }
        }
    }
}
