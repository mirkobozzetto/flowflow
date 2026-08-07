use crate::application::i18n::t;
use crate::application::transcribe_audio::transcribe_file;
use crate::application::transcription_manager::TranscriptionManager;
use crate::infrastructure::audio::{self, AudioRecorder, RecordingState};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::*;
use crate::ui::recording::Waveform;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const NUM_BARS: usize = 120;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Dictation only: the transcript goes back through `RecordingState` to
/// whatever input field is listening, and the file is always disposable. A kept
/// recording takes the durable `TranscriptionManager` path instead.
fn spawn_transcription(
    path: PathBuf,
    db: Arc<Database>,
    mut state: Signal<RecordingState>,
    generation: u64,
    gen_signal: Signal<u64>,
) {
    spawn(async move {
        let result = transcribe_file(&db, &path, true).await;
        if gen_signal() != generation {
            return;
        }
        match result {
            Ok(transcript) => {
                state.set(RecordingState::Transcribed { transcript })
            }
            Err(e) => state.set(RecordingState::Error(e)),
        }
    });
}

pub fn start_recording(
    recorder: Signal<Arc<Mutex<AudioRecorder>>>,
    mut app: AppState,
) {
    std::fs::create_dir_all(audio::output_dir()).ok();
    let rec = recorder();
    let mut rec = rec.lock().unwrap();
    match rec.start() {
        Ok(()) => app.recording_state.set(RecordingState::Recording),
        Err(e) => app.recording_state.set(RecordingState::Error(e)),
    }
}

#[component]
pub fn RecordingControls(
    pending_audio: Signal<Option<(String, f64)>>,
    #[props(default = false)] transcribe_only: bool,
) -> Element {
    let mut app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let manager: TranscriptionManager = use_context();
    let mut duration = use_signal(|| 0.0f32);
    let mut last_tap_ms = use_signal(|| 0u64);
    let mut transcription_gen = use_signal(|| 0u64);
    let lang = (app.current_lang)();
    let cancel_hint = t(&lang, "recording-cancel-hint");
    let pause_label = t(&lang, "recording-pause-label");
    let transcribing_label = t(&lang, "recording-transcribing");

    use_effect(move || {
        let state = (app.recording_state)();
        if state == RecordingState::Recording {
            let rec = recorder();
            spawn(async move {
                loop {
                    if (app.recording_state)() != RecordingState::Recording {
                        break;
                    }
                    let levels = rec.lock().unwrap().current_levels(NUM_BARS);
                    app.audio_levels.set(levels);
                    duration.set(rec.lock().unwrap().duration_secs());
                    futures_timer::Delay::new(
                        std::time::Duration::from_millis(60),
                    )
                    .await;
                }
            });
        }
    });

    use_effect(move || {
        let state = (app.recording_state)();
        if state == RecordingState::Recording || state == RecordingState::Paused
        {
            let rec = recorder();
            spawn(async move {
                loop {
                    let current = (app.recording_state)();
                    if current != RecordingState::Recording
                        && current != RecordingState::Paused
                    {
                        break;
                    }
                    let event = rec.lock().unwrap().poll_interruption();
                    if let Some(event) = event {
                        use crate::infrastructure::audio::InterruptionEvent;
                        match event {
                            InterruptionEvent::Began => {
                                rec.lock().unwrap().on_interruption_began();
                                app.recording_state.set(RecordingState::Paused);
                                app.audio_levels.set(vec![0.0; NUM_BARS]);
                            }
                            InterruptionEvent::Ended { should_resume } => {
                                let mut r = rec.lock().unwrap();
                                match r.on_interruption_ended(should_resume) {
                                    Ok(()) if should_resume => {
                                        drop(r);
                                        app.recording_state
                                            .set(RecordingState::Recording);
                                    }
                                    Err(e) => {
                                        drop(r);
                                        app.recording_state
                                            .set(RecordingState::Error(e));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    futures_timer::Delay::new(
                        std::time::Duration::from_millis(200),
                    )
                    .await;
                }
            });
        }
    });

    let recording_state = (app.recording_state)();
    let is_recording = recording_state == RecordingState::Recording;
    let is_paused = recording_state == RecordingState::Paused;
    let is_transcribing = recording_state == RecordingState::Transcribing;

    let secs = duration() as u32;
    let m = secs / 60;
    let s = secs % 60;

    if is_recording || is_paused {
        rsx! {
            div { class: "flex items-center gap-2",
                div { class: "flex-1 flex flex-col",
                    button {
                        class: if is_paused {
                            "flex items-center gap-3 h-12 px-4 rounded-full bg-stone-800 text-white text-sm overflow-hidden"
                        } else {
                            "flex items-center gap-3 h-12 px-4 rounded-full bg-stone-900 text-white text-sm shadow-lg overflow-hidden"
                        },
                        onclick: move |_| {
                            let now = now_ms();
                            let is_double = now - last_tap_ms() < 300;
                            last_tap_ms.set(now);
                            if is_double {
                                recorder().lock().unwrap().cancel();
                                app.recording_state.set(RecordingState::Idle);
                                duration.set(0.0);
                                app.audio_levels.set(vec![0.0; NUM_BARS]);
                            } else if is_recording {
                                recorder().lock().unwrap().pause();
                                app.recording_state.set(RecordingState::Paused);
                                app.audio_levels.set(vec![0.0; NUM_BARS]);
                            } else {
                                let rec = recorder();
                                let mut rec = rec.lock().unwrap();
                                match rec.resume() {
                                    Ok(()) => app.recording_state.set(RecordingState::Recording),
                                    Err(e) => app.recording_state.set(RecordingState::Error(e)),
                                }
                            }
                        },
                        if is_paused {
                            div { class: "flex gap-[2px] flex-shrink-0",
                                div { class: "w-[3px] h-3.5 bg-white/80 rounded-full" }
                                div { class: "w-[3px] h-3.5 bg-white/80 rounded-full" }
                            }
                        } else {
                            div { class: "w-2 h-2 rounded-full bg-ios-red flex-shrink-0",
                                style: "animation: pulseSoft 1.5s ease-in-out infinite;",
                            }
                        }
                        if is_paused {
                            span { class: "flex-1 text-center text-xs text-white/50", "{cancel_hint}" }
                        } else {
                            Waveform { num_bars: NUM_BARS, frozen: false }
                        }
                        span { class: "text-xs tabular-nums flex-shrink-0 ml-1",
                            {
                                let timer_label = if is_paused {
                                    format!("{pause_label} · {m}:{s:02}")
                                } else {
                                    format!("{m}:{s:02}")
                                };
                                rsx! { "{timer_label}" }
                            }
                        }
                    }
                }
                button {
                    class: "w-12 h-12 rounded-full bg-ios-orange text-white flex items-center justify-center flex-shrink-0",
                    onclick: move |_| {
                        let rec = recorder();
                        let mut rec = rec.lock().unwrap();
                        let dur = rec.duration_secs();
                        match rec.stop(&audio::output_dir()) {
                            Ok(path) => {
                                if transcribe_only {
                                    let gen = transcription_gen() + 1;
                                    transcription_gen.set(gen);
                                    app.recording_state.set(RecordingState::Transcribing);
                                    spawn_transcription(
                                        path, db(), app.recording_state, gen, transcription_gen,
                                    );
                                    return;
                                }
                                // The clip's id is known here, the moment it is stored. Carrying
                                // it through the job is what lets the word timings land on this
                                // exact clip rather than on whichever one happens to be last.
                                let filename = audio::audio_filename(&path.display().to_string());
                                if let Some(nid) = (app.current_note_id)().filter(|id| !id.is_empty()) {
                                    if let Ok(audio) = db().add_audio(&nid, &filename, dur as f64) {
                                        app.notes_version.set((app.notes_version)() + 1);
                                        manager.enqueue(nid, path, Some(audio.id));
                                    }
                                }
                                // `AudioJobBanner` is the progress UI from here on; leaving
                                // `Transcribing` set would freeze the recording bar, since
                                // nothing clears it on this path any more.
                                pending_audio.set(Some((filename, dur as f64)));
                                app.recording_state.set(RecordingState::Idle);
                            }
                            Err(e) => {
                                app.recording_state.set(RecordingState::Error(e));
                            }
                        }
                    },
                    IconPaperPlaneRight { size: 18 }
                }
            }
        }
    } else if is_transcribing {
        rsx! {
            div { class: "flex items-center justify-center",
                button {
                    class: "w-full flex items-center justify-center gap-2 h-12 rounded-full bg-stone-100 text-stone-400 text-sm",
                    onclick: move |_| {
                        let now = now_ms();
                        let is_double = now - last_tap_ms() < 300;
                        last_tap_ms.set(now);
                        if is_double {
                            transcription_gen.set(transcription_gen() + 1);
                            app.recording_state.set(RecordingState::Idle);
                        }
                    },
                    span { style: "animation: pulseSoft 1.5s ease-in-out infinite;", "{transcribing_label}" }
                }
            }
        }
    } else {
        rsx! {}
    }
}
