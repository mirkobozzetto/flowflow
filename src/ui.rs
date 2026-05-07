use crate::audio::{self, AudioRecorder, RecordingState};
use crate::soniox::SonioxClient;
use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn output_dir() -> String {
    let mut dir = std::env::temp_dir();
    dir.push("flowflow");
    dir.to_string_lossy().to_string()
}

fn start_transcription(path: PathBuf, mut state: Signal<RecordingState>) {
    spawn(async move {
        let client = match SonioxClient::from_env() {
            Ok(c) => c,
            Err(e) => {
                state.set(RecordingState::Error(e));
                return;
            }
        };
        match client.transcribe(&path).await {
            Ok(text) => {
                state.set(RecordingState::Transcribed(text));
            }
            Err(e) => {
                state.set(RecordingState::Error(e));
            }
        }
    });
}

#[component]
pub fn App() -> Element {
    let recorder = use_signal(|| Arc::new(Mutex::new(AudioRecorder::new())));
    let mut state = use_signal(|| RecordingState::Idle);
    let mut last_file = use_signal(String::new);
    let has_mic = use_signal(audio::has_input_device);

    rsx! {
        div {
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; font-family: -apple-system, BlinkMacSystemFont, sans-serif; gap: 24px; padding: 20px;",

            h1 {
                style: "font-size: 2rem; margin: 0;",
                "FlowFlow"
            }

            if has_mic() {
                RecordButton {
                    state: state(),
                    on_toggle: move |_| {
                        let rec = recorder();
                        let mut rec = rec.lock().unwrap();
                        let current = state();
                        match current {
                            RecordingState::Recording => {
                                match rec.stop(&output_dir()) {
                                    Ok(path) => {
                                        last_file.set(
                                            path.display()
                                                .to_string(),
                                        );
                                        state.set(
                                            RecordingState::Transcribing,
                                        );
                                        start_transcription(
                                            path, state,
                                        );
                                    }
                                    Err(e) => {
                                        state.set(
                                            RecordingState::Error(e),
                                        );
                                    }
                                }
                            }
                            _ => {
                                std::fs::create_dir_all(
                                    output_dir(),
                                )
                                .ok();
                                match rec.start() {
                                    Ok(()) => {
                                        state.set(
                                            RecordingState::Recording,
                                        );
                                    }
                                    Err(e) => {
                                        state.set(
                                            RecordingState::Error(e),
                                        );
                                    }
                                }
                            }
                        }
                    },
                }
            } else {
                p {
                    style: "font-size: 0.85rem; color: #999; text-align: center; max-width: 280px;",
                    "No microphone detected."
                }
            }

            StatusLine {
                state: state(),
                file: last_file(),
            }

        }
    }
}

#[component]
fn RecordButton(state: RecordingState, on_toggle: EventHandler<()>) -> Element {
    let is_recording = state == RecordingState::Recording;
    let bg = if is_recording { "#ff3b30" } else { "#34c759" };
    let radius = if is_recording { "4px" } else { "50%" };

    rsx! {
        button {
            style: "width: 80px; height: 80px; border-radius: 50%; border: none; cursor: pointer; background: {bg}; display: flex; align-items: center; justify-content: center;",
            onclick: move |_| on_toggle.call(()),
            div {
                style: "width: 24px; height: 24px; border-radius: {radius}; background: white;",
            }
        }
    }
}

#[component]
fn StatusLine(state: RecordingState, file: String) -> Element {
    let text = match &state {
        RecordingState::Idle => "Tap to record".to_string(),
        RecordingState::Recording => "Recording...".to_string(),
        RecordingState::Stopped(_) => {
            format!("Saved: {file}")
        }
        RecordingState::Transcribing => "Transcribing...".to_string(),
        RecordingState::Transcribed(text) => text.clone(),
        RecordingState::Error(e) => {
            format!("Error: {e}")
        }
    };

    rsx! {
        p {
            style: "font-size: 0.9rem; color: #888; margin: 0; text-align: center; word-break: break-all; max-width: 320px;",
            "{text}"
        }
    }
}
