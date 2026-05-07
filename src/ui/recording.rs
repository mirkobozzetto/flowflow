use crate::services::audio::{AudioRecorder, RecordingState};
use crate::services::transcription::SonioxClient;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn output_dir() -> String {
    #[cfg(target_os = "ios")]
    {
        let dir = crate::platform::ios::documents_dir().join("flowflow");
        std::fs::create_dir_all(&dir).ok();
        dir.to_string_lossy().to_string()
    }
    #[cfg(not(target_os = "ios"))]
    {
        let mut dir = std::env::temp_dir();
        dir.push("flowflow");
        std::fs::create_dir_all(&dir).ok();
        dir.to_string_lossy().to_string()
    }
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
            Ok(text) => state.set(RecordingState::Transcribed(text)),
            Err(e) => state.set(RecordingState::Error(e)),
        }
    });
}

#[component]
pub fn RecordingView() -> Element {
    let mut app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let mut last_file = use_signal(String::new);

    rsx! {
        div { class: "flex-1 flex flex-col items-center justify-center gap-6 px-5",
            RecordButton {
                state: (app.recording_state)(),
                on_toggle: move |_| {
                    let rec = recorder();
                    let mut rec = rec.lock().unwrap();
                    let current = (app.recording_state)();
                    match current {
                        RecordingState::Recording => {
                            match rec.stop(&output_dir()) {
                                Ok(path) => {
                                    last_file.set(path.display().to_string());
                                    app.recording_state.set(RecordingState::Transcribing);
                                    start_transcription(path, app.recording_state);
                                }
                                Err(e) => app.recording_state.set(RecordingState::Error(e)),
                            }
                        }
                        _ => {
                            std::fs::create_dir_all(output_dir()).ok();
                            match rec.start() {
                                Ok(()) => app.recording_state.set(RecordingState::Recording),
                                Err(e) => app.recording_state.set(RecordingState::Error(e)),
                            }
                        }
                    }
                },
            }
            StatusLine { state: (app.recording_state)(), file: last_file() }
        }
    }
}

#[component]
fn RecordButton(state: RecordingState, on_toggle: EventHandler<()>) -> Element {
    let is_recording = state == RecordingState::Recording;
    let btn_class = if is_recording {
        "w-20 h-20 rounded-full border-none bg-ios-red flex items-center justify-center"
    } else {
        "w-20 h-20 rounded-full border-none bg-ios-green flex items-center justify-center"
    };
    let icon_class = if is_recording {
        "w-6 h-6 rounded bg-white"
    } else {
        "w-6 h-6 rounded-full bg-white"
    };

    rsx! {
        button {
            class: "{btn_class}",
            onclick: move |_| on_toggle.call(()),
            div { class: "{icon_class}" }
        }
    }
}

#[component]
fn StatusLine(state: RecordingState, file: String) -> Element {
    let text = match &state {
        RecordingState::Idle => "Appuyez pour enregistrer".to_string(),
        RecordingState::Recording => "Enregistrement...".to_string(),
        RecordingState::Stopped(_) => format!("Sauvegardé : {file}"),
        RecordingState::Transcribing => "Transcription...".to_string(),
        RecordingState::Transcribed(text) => text.clone(),
        RecordingState::Error(e) => format!("Erreur : {e}"),
    };

    rsx! {
        p { class: "text-sm text-gray-400 text-center break-all max-w-xs", "{text}" }
    }
}
