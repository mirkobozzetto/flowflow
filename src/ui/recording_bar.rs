use crate::db::Database;
use crate::services::audio::{self, AudioRecorder, RecordingState};
use crate::services::transcription::SonioxClient;
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
pub fn RecordingBar() -> Element {
    let mut app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let mut last_audio_path = use_signal(String::new);
    let mut duration = use_signal(|| 0.0f32);

    use_effect(move || {
        let state = (app.recording_state)();
        if state == RecordingState::Recording {
            let rec = recorder();
            spawn(async move {
                loop {
                    if (app.recording_state)() != RecordingState::Recording {
                        app.audio_levels.set(vec![0.0; 12]);
                        break;
                    }
                    let levels = rec.lock().unwrap().current_levels(12);
                    app.audio_levels.set(levels);
                    duration.set(rec.lock().unwrap().duration_secs());
                    futures_timer::Delay::new(
                        std::time::Duration::from_millis(80),
                    )
                    .await;
                }
            });
        }
    });

    let recording_state = (app.recording_state)();
    let is_recording = recording_state == RecordingState::Recording;
    let is_transcribing = recording_state == RecordingState::Transcribing;

    rsx! {
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-3 bg-white border-t border-gray-200 z-30 keyboard-aware",
            div { class: "flex items-center justify-center",
                button {
                    class: if is_transcribing {
                        "w-full flex items-center justify-center gap-2 h-12 rounded-full bg-gray-100 text-gray-400 text-sm"
                    } else if is_recording {
                        "w-full flex items-center justify-center gap-3 h-12 rounded-full bg-gray-900 text-white text-sm shadow-lg"
                    } else {
                        "w-full flex items-center justify-center gap-2.5 h-12 rounded-full bg-ios-blue/10 text-ios-blue text-sm font-medium"
                    },
                    disabled: is_transcribing,
                    onclick: move |_| {
                        let rec = recorder();
                        let mut rec = rec.lock().unwrap();
                        let current_state = (app.recording_state)();
                        match current_state {
                            RecordingState::Recording => {
                                let dur = rec.duration_secs();
                                match rec.stop(&audio::output_dir()) {
                                    Ok(path) => {
                                        let path_str = path.display().to_string();
                                        last_audio_path.set(path_str.clone());
                                        if let Some(ref nid) = (app.current_note_id)() {
                                            let _ = db().update_audio_metadata(
                                                nid,
                                                &path_str,
                                                dur as f64,
                                            );
                                        }
                                        app.recording_state.set(
                                            RecordingState::Transcribing,
                                        );
                                        start_transcription(
                                            path,
                                            app.recording_state,
                                        );
                                    }
                                    Err(e) => {
                                        app.recording_state.set(
                                            RecordingState::Error(e),
                                        );
                                    }
                                }
                            }
                            _ => {
                                std::fs::create_dir_all(audio::output_dir())
                                    .ok();
                                match rec.start() {
                                    Ok(()) => {
                                        app.recording_state.set(
                                            RecordingState::Recording,
                                        );
                                    }
                                    Err(e) => {
                                        app.recording_state.set(
                                            RecordingState::Error(e),
                                        );
                                    }
                                }
                            }
                        }
                    },
                    if is_recording {
                        div { class: "w-2 h-2 rounded-full bg-ios-red" }
                        {
                            let levels = (app.audio_levels)();
                            rsx! {
                                div { class: "flex items-end gap-[3px] h-5",
                                    for (i, &lvl) in levels.iter().enumerate() {
                                        {
                                            let h = 3.0 + lvl * 17.0;
                                            let key = format!("bar-{i}");
                                            rsx! {
                                                div {
                                                    key: "{key}",
                                                    class: "w-[3px] bg-white rounded-full transition-all duration-75",
                                                    style: "height: {h:.0}px;",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        {
                            let secs = duration() as u32;
                            let m = secs / 60;
                            let s = secs % 60;
                            rsx! { span { "{m}:{s:02}" } }
                        }
                    } else if is_transcribing {
                        span { style: "animation: pulseSoft 1.5s ease-in-out infinite;", "Transcription..." }
                    } else {
                        IconMic { size: 22 }
                        span { "Dicter" }
                    }
                }
            }
        }
    }
}
