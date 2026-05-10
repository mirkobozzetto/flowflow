use crate::services::audio::{self, AudioRecorder, RecordingState};
use crate::services::transcription::SonioxClient;
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

fn start_transcription(
    path: std::path::PathBuf,
    mut state: Signal<RecordingState>,
) {
    spawn(async move {
        let client = match SonioxClient::from_env() {
            Ok(c) => c,
            Err(e) => {
                state.set(RecordingState::Error(e));
                return;
            }
        };
        match client.transcribe(&path).await {
            Ok(text) => state.set(RecordingState::Transcribed(text)),
            Err(e) => state.set(RecordingState::Error(e)),
        }
    });
}

#[component]
pub fn ChatInputBar(
    input: Signal<String>,
    disabled: bool,
    on_send: EventHandler<String>,
) -> Element {
    let mut app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();

    let recording_state = (app.recording_state)();
    let is_recording = recording_state == RecordingState::Recording;
    let is_transcribing = recording_state == RecordingState::Transcribing;

    rsx! {
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-3 bg-white border-t border-gray-200 z-30 keyboard-aware",
            div { class: "flex items-center gap-2",
                button {
                    class: if is_recording {
                        "w-10 h-10 rounded-full bg-ios-red/10 flex items-center justify-center text-ios-red transition-colors duration-150"
                    } else {
                        "w-10 h-10 rounded-full bg-gray-100 flex items-center justify-center text-gray-500 transition-colors duration-150"
                    },
                    disabled: is_transcribing || disabled,
                    onclick: move |_| {
                        let rec = recorder();
                        let mut rec = rec.lock().unwrap();
                        if is_recording {
                            match rec.stop(&audio::output_dir()) {
                                Ok(path) => {
                                    app.recording_state
                                        .set(RecordingState::Transcribing);
                                    start_transcription(
                                        path,
                                        app.recording_state,
                                    );
                                }
                                Err(e) => {
                                    app.recording_state
                                        .set(RecordingState::Error(e));
                                }
                            }
                        } else {
                            std::fs::create_dir_all(audio::output_dir()).ok();
                            match rec.start() {
                                Ok(()) => {
                                    app.recording_state
                                        .set(RecordingState::Recording);
                                }
                                Err(e) => {
                                    app.recording_state
                                        .set(RecordingState::Error(e));
                                }
                            }
                        }
                    },
                    if is_recording {
                        IconStop { size: 18 }
                    } else {
                        IconMic { size: 18 }
                    }
                }
                input {
                    class: "flex-1 bg-gray-100 rounded-full px-4 py-2.5 text-sm outline-none text-gray-900 placeholder-gray-400",
                    placeholder: if is_transcribing { "Transcription..." } else { "Pose une question..." },
                    value: "{input}",
                    disabled: is_transcribing,
                    oninput: move |evt| input.set(evt.value()),
                }
                button {
                    class: if disabled || input().trim().is_empty() {
                        "w-10 h-10 rounded-full bg-gray-200 flex items-center justify-center text-gray-400 transition-colors duration-150"
                    } else {
                        "w-10 h-10 rounded-full bg-ios-blue flex items-center justify-center text-white transition-colors duration-150"
                    },
                    disabled: disabled || input().trim().is_empty(),
                    onclick: move |_| {
                        let q = input().trim().to_string();
                        if !q.is_empty() && !disabled {
                            input.set(String::new());
                            on_send.call(q);
                        }
                    },
                    IconPaperPlaneRight { size: 16 }
                }
            }
            if is_transcribing {
                p {
                    class: "text-xs text-gray-400 text-center mt-2",
                    style: "animation: pulseSoft 1.5s ease-in-out infinite;",
                    "Transcription en cours..."
                }
            }
            if is_recording {
                p { class: "text-xs text-ios-red text-center mt-2",
                    "Enregistrement..."
                }
            }
            if let RecordingState::Error(ref e) = recording_state {
                p { class: "text-xs text-ios-red text-center mt-2",
                    "{e}"
                }
            }
        }
    }
}
