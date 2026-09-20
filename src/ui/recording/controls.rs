use crate::application::i18n::t;
use crate::application::transcribe_audio::transcribe_file;
use crate::application::transcription_manager::TranscriptionManager;
use crate::infrastructure::audio::{self, AudioRecorder, RecordingState};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::{haptic, haptic_prepare};
use crate::ui::icons::*;
use crate::ui::recording::waveform::{mount_timeline, TICK_MS};
use crate::ui::recording::Waveform;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

// The dark capsule that replaces the composer while a take is live. Both roles
// share it; only the arrow differs: chat transcribes then sends, a note keeps
// the clip and its durable transcription job (unchanged path). The square stops
// and transcribes for review in the field; X cancels (replaces double-tap).
#[component]
pub fn VoiceCapsule(
    pending_audio: Signal<Option<(String, f64)>>,
    #[props(default = false)] transcribe_only: bool,
    commit_on_transcribed: Signal<bool>,
) -> Element {
    let mut app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let manager: TranscriptionManager = use_context();
    let mut duration = use_signal(|| 0.0f32);
    let mut transcription_gen = use_signal(|| 0u64);
    let lang = (app.current_lang)();
    let cancel_label = t(&lang, "recording-cancel");
    let stop_label = t(&lang, "recording-stop");
    let send_label = t(&lang, "recording-send");
    let pause_label = t(&lang, "recording-pause-label");
    let transcribing_label = t(&lang, "recording-transcribing");

    use_effect(move || {
        let state = (app.recording_state)();
        if state == RecordingState::Recording {
            let rec = recorder();
            spawn(async move {
                // One controller per take segment: a resume re-installs the
                // loop over the bars that are still in the DOM.
                let timeline = mount_timeline();
                loop {
                    if (app.recording_state)() != RecordingState::Recording {
                        break;
                    }
                    let (rms, peak) = rec
                        .lock()
                        .unwrap()
                        .recent_levels(TICK_MS as f32 / 1000.0);
                    let _ = timeline.send((rms, peak));
                    duration.set(rec.lock().unwrap().duration_secs());
                    futures_timer::Delay::new(
                        std::time::Duration::from_millis(TICK_MS),
                    )
                    .await;
                }
                let _ = timeline.send(());
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
    let is_paused = recording_state == RecordingState::Paused;
    let is_transcribing = recording_state == RecordingState::Transcribing;
    let live = !is_transcribing;

    let secs = duration() as u32;
    let timer = format!("{}:{:02}", secs / 60, secs % 60);

    let reset = use_callback(move |()| {
        let mut duration = duration;
        duration.set(0.0);
    });

    // Square: stop, transcribe the disposable file, land the text in the field.
    // Arrow: chat transcribes then commits; a note stores the clip as before.
    let finish = use_callback(move |commit: bool| {
        let mut app = app;
        let rec = recorder();
        let mut rec = rec.lock().unwrap();
        let dur = rec.duration_secs();
        match rec.stop(&audio::output_dir()) {
            Ok(path) => {
                drop(rec);
                reset(());
                let mut transcription_gen = transcription_gen;
                let mut commit_on_transcribed = commit_on_transcribed;
                if transcribe_only || !commit {
                    let gen = transcription_gen() + 1;
                    transcription_gen.set(gen);
                    commit_on_transcribed.set(commit);
                    app.recording_state.set(RecordingState::Transcribing);
                    spawn_transcription(
                        path,
                        db(),
                        app.recording_state,
                        gen,
                        transcription_gen,
                    );
                    return;
                }
                // The clip's id is known here, the moment it is stored. Carrying
                // it through the job is what lets the word timings land on this
                // exact clip rather than on whichever one happens to be last.
                let filename =
                    audio::audio_filename(&path.display().to_string());
                if let Some(nid) =
                    (app.current_note_id)().filter(|id| !id.is_empty())
                {
                    if let Ok(audio) =
                        db().add_audio(&nid, &filename, dur as f64)
                    {
                        app.notes_version.set((app.notes_version)() + 1);
                        manager.enqueue(nid, path, Some(audio.id));
                    }
                }
                // `AudioJobBanner` is the progress UI from here on; leaving
                // `Transcribing` set would freeze the bar, since nothing clears
                // it on this path any more.
                pending_audio.set(Some((filename, dur as f64)));
                app.recording_state.set(RecordingState::Idle);
            }
            Err(e) => app.recording_state.set(RecordingState::Error(e)),
        }
    });

    rsx! {
        div {
            class: "voice-capsule flex items-center gap-1.5 px-1.5 min-h-14 rounded-full bg-stone-900 text-white shadow-lift overflow-hidden",
            role: "group",
            "aria-label": t(&lang, "recording-dictate"),
            button {
                class: "pressable w-11 h-11 shrink-0 rounded-full border border-white/20 flex items-center justify-center disabled:opacity-40",
                "aria-label": "{cancel_label}",
                disabled: !live,
                onclick: move |_| {
                    if is_transcribing {
                        transcription_gen.set(transcription_gen() + 1);
                    } else {
                        recorder().lock().unwrap().cancel();
                    }
                    reset(());
                    app.recording_state.set(RecordingState::Idle);
                },
                IconX { size: 18 }
            }
            if is_transcribing {
                span { class: "flex-1 text-center text-[13px] text-white/60", style: "animation: pulseSoft 1.5s ease-in-out infinite;", "{transcribing_label}" }
            } else {
                button {
                    class: "flex-1 min-w-0 flex items-center gap-2 h-11 text-left",
                    "aria-label": if is_paused { "{pause_label} · {timer}" } else { "{timer}" },
                    onclick: move |_| {
                        let rec = recorder();
                        let mut rec = rec.lock().unwrap();
                        if is_paused {
                            match rec.resume() {
                                Ok(()) => app.recording_state.set(RecordingState::Recording),
                                Err(e) => app.recording_state.set(RecordingState::Error(e)),
                            }
                        } else {
                            rec.pause();
                            drop(rec);
                            app.recording_state.set(RecordingState::Paused);
                                            }
                    },
                    Waveform {}
                    span { class: "text-xs tabular-nums shrink-0 text-white/80",
                        if is_paused { "{pause_label} · {timer}" } else { "{timer}" }
                    }
                }
            }
            button {
                class: "pressable w-11 h-11 shrink-0 rounded-full bg-white/15 flex items-center justify-center disabled:opacity-40",
                "aria-label": "{stop_label}",
                disabled: !live,
                onpointerdown: move |_| haptic_prepare("light"),
                onclick: move |_| { haptic("light"); finish(false); },
                span { class: "block w-[13px] h-[13px] rounded-[3px] bg-white" }
            }
            button {
                class: "pressable w-11 h-11 shrink-0 rounded-full bg-ios-orange flex items-center justify-center disabled:opacity-40",
                "aria-label": "{send_label}",
                disabled: !live,
                onpointerdown: move |_| haptic_prepare("soft"),
                onclick: move |_| { haptic("soft"); finish(true); },
                IconArrowUp { size: 20 }
            }
        }
    }
}
