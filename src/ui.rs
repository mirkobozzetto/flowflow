use crate::audio::{self, AudioRecorder, RecordingState};
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

const OUTPUT_DIR: &str = "/tmp/flowflow";

#[component]
pub fn App() -> Element {
    let recorder = use_signal(|| Arc::new(Mutex::new(AudioRecorder::new())));
    let mut state = use_signal(|| RecordingState::Idle);
    let mut last_file = use_signal(|| String::new());
    let has_mic = use_signal(|| audio::has_input_device());

    rsx! {
        div {
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; font-family: -apple-system, BlinkMacSystemFont, sans-serif; gap: 24px; padding: 20px;",

            h1 { style: "font-size: 2rem; margin: 0;", "FlowFlow" }

            if has_mic() {
                RecordButton {
                    state: state(),
                    on_toggle: move |_| {
                        let rec = recorder();
                        let mut rec = rec.lock().unwrap();
                        let current = state();
                        match current {
                            RecordingState::Recording => {
                                match rec.stop(OUTPUT_DIR) {
                                    Ok(path) => {
                                        last_file.set(path.display().to_string());
                                        state.set(RecordingState::Stopped(path));
                                    }
                                    Err(e) => state.set(RecordingState::Error(e)),
                                }
                            }
                            _ => {
                                std::fs::create_dir_all(OUTPUT_DIR).ok();
                                match rec.start() {
                                    Ok(()) => state.set(RecordingState::Recording),
                                    Err(e) => state.set(RecordingState::Error(e)),
                                }
                            }
                        }
                    },
                }
            } else {
                p {
                    style: "font-size: 0.85rem; color: #999; text-align: center; max-width: 280px;",
                    "No microphone detected (simulator). Use \"Generate test WAV\" to test the pipeline."
                }
            }

            StatusLine { state: state(), file: last_file() }

            if !matches!(state(), RecordingState::Recording) {
                TestWavButton {
                    on_generate: move |_| {
                        std::fs::create_dir_all(OUTPUT_DIR).ok();
                        match audio::generate_test_wav(OUTPUT_DIR) {
                            Ok(path) => {
                                last_file.set(path.display().to_string());
                                state.set(RecordingState::Stopped(path));
                            }
                            Err(e) => state.set(RecordingState::Error(e)),
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn RecordButton(state: RecordingState, on_toggle: EventHandler<()>) -> Element {
    let is_recording = state == RecordingState::Recording;
    let bg = if is_recording { "#ff3b30" } else { "#34c759" };
    let icon = if is_recording { "⏹" } else { "⏺" };

    rsx! {
        button {
            style: "width: 80px; height: 80px; border-radius: 50%; border: none; font-size: 2rem; cursor: pointer; background: {bg}; color: white;",
            onclick: move |_| on_toggle.call(()),
            "{icon}"
        }
    }
}

#[component]
fn StatusLine(state: RecordingState, file: String) -> Element {
    let text = match &state {
        RecordingState::Idle => "Tap to record".to_string(),
        RecordingState::Recording => "Recording...".to_string(),
        RecordingState::Stopped(_) => format!("Saved: {file}"),
        RecordingState::Error(e) => format!("Error: {e}"),
    };

    rsx! {
        p { style: "font-size: 0.9rem; color: #888; margin: 0; text-align: center; word-break: break-all;", "{text}" }
    }
}

#[component]
fn TestWavButton(on_generate: EventHandler<()>) -> Element {
    rsx! {
        button {
            style: "padding: 12px 24px; border: 1px solid #ccc; border-radius: 8px; background: none; font-size: 0.9rem; cursor: pointer;",
            onclick: move |_| on_generate.call(()),
            "Generate test WAV (sine 440Hz)"
        }
    }
}
