use crate::application::i18n::t;
use crate::application::transcribe_audio::transcribe_file;
use crate::application::transcription_manager::{
    JobStatus, TranscriptionManager,
};
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
    // A note's arrow hands the take to the durable job: (note id, clip id)
    // keeps the capsule on screen until that job's text is in the note.
    let mut tracked: Signal<Option<(String, String)>> = use_signal(|| None);
    let lang = (app.current_lang)();
    let cancel_label = t(&lang, "recording-cancel");
    let stop_label = t(&lang, "recording-stop");
    let send_label = t(&lang, "recording-send");
    let pause_label = t(&lang, "recording-pause-label");
    let transcribing_label = t(&lang, "recording-transcribing");
    let failed_label = t(&lang, "audio-transcription-failed");
    let retry_label = t(&lang, "audio-import-retry");

    use_effect(move || {
        let state = (app.recording_state)();
        if state == RecordingState::Recording {
            let rec = recorder();
            spawn(async move {
                // One controller per take segment: a resume re-installs the
                // loop over the bars that are still in the DOM.
                let timeline = mount_timeline();
                // Log what the webview really drew (see voice_timeline.ts).
                spawn({
                    let mut probe = timeline.clone();
                    async move {
                        while let Ok(msg) = probe.recv::<String>().await {
                            eprintln!("[voice] {msg}");
                        }
                    }
                });
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

    // The job leaves the queue once the note has merged its text: that is
    // when the pill comes back. The manager is the truth, the signal the tick.
    let job_manager = manager.clone();
    let retry_manager = manager.clone();
    use_effect(move || {
        let _ = (app.transcription_jobs)();
        let Some((nid, aid)) = tracked.peek().clone() else {
            return;
        };
        let pending = job_manager.snapshot().get(&nid).is_some_and(|q| {
            q.iter()
                .any(|j| j.audio_id.as_deref() == Some(aid.as_str()))
        });
        if !pending {
            tracked.set(None);
            haptic("light");
            app.recording_state.set(RecordingState::Idle);
        }
    });

    // Leaving the note mid-job: the job carries on (banner, watcher), but the
    // shared recording state must not keep the next composer hidden.
    use_drop(move || {
        let was_tracking =
            tracked.try_peek().map(|t| t.is_some()).unwrap_or(false);
        if was_tracking {
            app.recording_state.set(RecordingState::Idle);
        }
    });

    let failed: Option<String> = tracked().and_then(|(nid, aid)| {
        (app.transcription_jobs)().get(&nid).and_then(|q| {
            q.iter().find_map(|j| match &j.status {
                JobStatus::Failed(reason)
                    if j.audio_id.as_deref() == Some(aid.as_str()) =>
                {
                    Some(reason.clone())
                }
                _ => None,
            })
        })
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
                        manager.enqueue(
                            nid.clone(),
                            path,
                            Some(audio.id.clone()),
                        );
                        let mut tracked = tracked;
                        tracked.set(Some((nid, audio.id)));
                    }
                }
                pending_audio.set(Some((filename, dur as f64)));
                // Without a job to follow (note not created yet), the note's
                // own banner takes over as before.
                app.recording_state.set(if tracked.peek().is_some() {
                    RecordingState::Transcribing
                } else {
                    RecordingState::Idle
                });
            }
            Err(e) => app.recording_state.set(RecordingState::Error(e)),
        }
    });

    rsx! {
        div {
            class: "voice-capsule h-full flex items-center gap-1.5 px-1 min-h-14 rounded-full bg-stone-900 text-white shadow-lift overflow-hidden",
            "data-transcribing": is_transcribing,
            role: "group",
            "aria-label": t(&lang, "recording-dictate"),
            button {
                class: "voice-ghost pressable w-11 h-11 shrink-0 rounded-full border border-white/20 flex items-center justify-center",
                "aria-label": "{cancel_label}",
                // Dictation: drop the result. A note's job: stop watching it,
                // it finishes in the background under the note's banner.
                onclick: move |_| {
                    if tracked.peek().is_some() {
                        tracked.set(None);
                    } else if is_transcribing {
                        transcription_gen.set(transcription_gen() + 1);
                    } else {
                        recorder().lock().unwrap().cancel();
                    }
                    reset(());
                    app.recording_state.set(RecordingState::Idle);
                },
                IconX { size: 18 }
            }
            if let Some(reason) = failed {
                span { class: "flex-1 min-w-0 truncate pl-1 text-[13px] text-ios-red", title: "{reason}", "{failed_label}" }
                button {
                    class: "pressable press-grow h-10 px-3.5 shrink-0 rounded-full bg-white/15 text-[13px] font-semibold",
                    onpointerdown: move |_| haptic_prepare("light"),
                    onclick: move |_| {
                        haptic("light");
                        if let Some((nid, _)) = tracked.peek().clone() {
                            retry_manager.retry(&nid);
                        }
                    },
                    "{retry_label}"
                }
            } else {
                // Kept mounted while transcribing: the take's bars stay
                // frozen in place and breathe (CSS) instead of vanishing.
                button {
                    class: "flex-1 min-w-0 flex items-center gap-2 h-11 text-left",
                    "aria-label": if is_transcribing { "{transcribing_label}" } else if is_paused { "{pause_label} · {timer}" } else { "{timer}" },
                    disabled: is_transcribing,
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
                    span { class: "text-xs tabular-nums shrink-0 text-white/80 pr-1",
                        if is_transcribing { "{transcribing_label}" } else if is_paused { "{pause_label} · {timer}" } else { "{timer}" }
                    }
                }
                if live {
                    button {
                        class: "pressable press-grow w-12 h-12 shrink-0 rounded-full bg-white/15 flex items-center justify-center",
                        "aria-label": "{stop_label}",
                        onpointerdown: move |_| haptic_prepare("light"),
                        onclick: move |_| { haptic("light"); finish(false); },
                        span { class: "block w-[14px] h-[14px] rounded-[3px] bg-white" }
                    }
                }
                button {
                    class: "pressable press-grow w-12 h-12 shrink-0 rounded-full bg-ios-orange flex items-center justify-center",
                    "aria-label": if is_transcribing { "{transcribing_label}" } else { "{send_label}" },
                    disabled: !live,
                    onpointerdown: move |_| haptic_prepare("soft"),
                    onclick: move |_| { haptic("soft"); finish(true); },
                    if is_transcribing {
                        span { class: "block w-[18px] h-[18px] rounded-full border-2 border-white/35 border-t-white animate-spin" }
                    } else {
                        IconArrowUp { size: 20 }
                    }
                }
            }
        }
    }
}
