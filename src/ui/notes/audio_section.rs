use crate::application::i18n::{t, t_args};
use crate::application::transcribe_audio::retranscribe_audio;
use crate::application::transcription_manager::{
    JobStatus, TranscriptionManager,
};
use crate::domain::NoteAudio;
use crate::domain::Transcript;
use crate::infrastructure::audio;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::IconX;
use crate::ui::notes::audio_player::AudioPlayer;
use crate::ui::notes::dates::format_relative_date;
use crate::ui::notes::transcript_view::TranscriptView;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

/// Fast enough that a word boundary is never missed at speech rate, cheap enough
/// that it only costs an atomic read: the position is counted in Rust, so the
/// tick touches no platform API.
const TICK_MS: u64 = 100;

#[derive(Clone, PartialEq)]
struct Playback {
    audio_id: String,
    transcript: Transcript,
    duration_secs: f64,
}

fn playhead_secs() -> Option<f64> {
    #[cfg(target_os = "ios")]
    {
        crate::infrastructure::platform::ios::current_time_secs()
    }
    // Desktop plays through a detached `afplay`, which exposes no position
    // (RFC 0024 non-goal). The transcript renders, inert.
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

fn start_playback(path: &str, from_secs: f64) {
    #[cfg(target_os = "ios")]
    crate::infrastructure::platform::ios::play_audio_at(path, from_secs);
    #[cfg(target_os = "macos")]
    {
        let _ = from_secs;
        crate::infrastructure::platform::macos::play_audio(path);
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        let _ = (path, from_secs);
    }
}

fn stop_playback() {
    #[cfg(target_os = "ios")]
    crate::infrastructure::platform::ios::stop_audio();
    #[cfg(target_os = "macos")]
    crate::infrastructure::platform::macos::stop_audio();
}

/// Move a running clip, restarting it when the platform cannot seek in place.
fn seek_playback(path: &str, to_secs: f64) {
    #[cfg(target_os = "ios")]
    {
        if crate::infrastructure::platform::ios::seek_to(to_secs) {
            return;
        }
    }
    start_playback(path, to_secs);
}

#[component]
pub fn AudioSection(
    local_note_id: Signal<String>,
    mut audios_version: Signal<u32>,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let audios: Vec<NoteAudio> = {
        let id = local_note_id();
        let _ = audios_version();
        let _ = (app.notes_version)();
        if id.is_empty() {
            Vec::new()
        } else {
            db().list_audios(&id).unwrap_or_default()
        }
    };
    let mut audios_expanded = use_signal(|| false);
    let mut transcribing_audio_id: Signal<Option<String>> = use_signal(|| None);
    let mut transcribe_error: Signal<Option<(String, String)>> =
        use_signal(|| None);

    // One clip plays at a time, so playback state lives here rather than in each
    // player row - which is also what lets the transcript follow the audio.
    let mut playback: Signal<Option<Playback>> = use_signal(|| None);
    let mut active_word: Signal<Option<usize>> = use_signal(|| None);

    use_future(move || async move {
        loop {
            futures_timer::Delay::new(std::time::Duration::from_millis(
                TICK_MS,
            ))
            .await;
            let Some(current) = playback.peek().clone() else {
                if active_word.peek().is_some() {
                    active_word.set(None);
                }
                continue;
            };
            let Some(elapsed) = playhead_secs() else {
                continue;
            };
            if current.duration_secs > 0.0 && elapsed >= current.duration_secs {
                playback.set(None);
                active_word.set(None);
                continue;
            }
            // Written only when the index changes: at normal speech that is two
            // or three re-renders a second rather than ten.
            let index = current
                .transcript
                .word_index_at_ms((elapsed * 1000.0) as u32);
            if *active_word.peek() != index {
                active_word.set(index);
            }
        }
    });

    let transcribing_short = t(&lang, "note-transcribing-short");
    let transcribe_action = t(&lang, "note-transcribe-action");

    rsx! {
        if !audios.is_empty() {
            {
                let count = audios.len().to_string();
                let audio_label = t_args(&lang, "note-audios-label", &[("count", &count)]);
                rsx! {
                    div { class: "mt-3",
                        button {
                            class: "flex items-center gap-2 w-full py-2 text-left active:opacity-70 hover:opacity-70 transition-opacity duration-150",
                            onclick: move |_| audios_expanded.set(!audios_expanded()),
                            span { class: "text-xs font-medium text-stone-500", "{audio_label}" }
                            span {
                                class: "inline-block w-1.5 h-1.5 border-r-[1.5px] border-b-[1.5px] border-stone-400 chevron-pivot",
                                class: if audios_expanded() { "-rotate-[135deg]" } else { "rotate-45" },
                            }
                        }
                        if audios_expanded() {
                            div { class: "space-y-2 pt-1",
                                for audio in audios.iter() {
                                    {
                                        let audio_id = audio.id.clone();
                                        let audio_id_tr = audio.id.clone();
                                        let file_path = audio.file_path.clone();
                                        let file_path_tr = audio.file_path.clone();
                                        let resolved = audio::resolve_audio_path(&file_path);
                                        let file_exists = std::path::Path::new(&resolved).exists();
                                        let date = format_relative_date(&audio.created_at, &lang);
                                        let transcription = audio.transcription.clone();
                                        let transcript = db().words_for_audio(&audio.id).unwrap_or_default();
                                        let is_playing_this = playback().map(|p| p.audio_id) == Some(audio.id.clone());
                                        let highlighted = if is_playing_this { active_word() } else { None };
                                        let audio_id_play = audio.id.clone();
                                        let audio_id_seek = audio.id.clone();
                                        let path_play = resolved.clone();
                                        let path_seek = resolved.clone();
                                        let transcript_play = transcript.clone();
                                        let transcript_seek = transcript.clone();
                                        let duration_play = audio.duration_secs.unwrap_or(0.0);
                                        let is_transcribing_this = transcribing_audio_id() == Some(audio.id.clone());
                                        let error_msg = transcribe_error()
                                            .filter(|(id, _)| *id == audio.id)
                                            .map(|(_, msg)| msg);
                                        rsx! {
                                            div { class: "space-y-1",
                                                p { class: "text-xs text-stone-500 px-1", "{date}" }
                                                AudioPlayer {
                                                    audio_path: resolved,
                                                    duration_secs: audio.duration_secs,
                                                    playing: is_playing_this,
                                                    on_play: move |_| {
                                                        start_playback(&path_play, 0.0);
                                                        active_word.set(None);
                                                        playback.set(Some(Playback {
                                                            audio_id: audio_id_play.clone(),
                                                            transcript: transcript_play.clone(),
                                                            duration_secs: duration_play,
                                                        }));
                                                    },
                                                    on_stop: move |_| {
                                                        stop_playback();
                                                        playback.set(None);
                                                        active_word.set(None);
                                                    },
                                                    on_delete: move |_| {
                                                        let fp = file_path.clone();
                                                        let _ = std::fs::remove_file(audio::resolve_audio_path(&fp));
                                                        let _ = db().delete_audio(&audio_id);
                                                        audios_version.set(audios_version() + 1);
                                                        app.notes_version.set((app.notes_version)() + 1);
                                                    },
                                                }
                                                if !transcript.is_empty() {
                                                    TranscriptView {
                                                        words: transcript.words.clone(),
                                                        active: highlighted,
                                                        seekable: cfg!(target_os = "ios") && file_exists,
                                                        on_seek: move |secs: f64| {
                                                            if is_playing_this {
                                                                seek_playback(&path_seek, secs);
                                                            } else {
                                                                start_playback(&path_seek, secs);
                                                                playback.set(Some(Playback {
                                                                    audio_id: audio_id_seek.clone(),
                                                                    transcript: transcript_seek.clone(),
                                                                    duration_secs: duration_play,
                                                                }));
                                                            }
                                                        },
                                                    }
                                                } else if let Some(ref text) = transcription {
                                                    p { class: "text-xs text-stone-600 italic px-1 pb-1", "{text}" }
                                                } else if is_transcribing_this {
                                                    p {
                                                        class: "text-xs text-stone-400 px-1 pb-1",
                                                        style: "animation: pulseSoft 1.5s ease-in-out infinite;",
                                                        "{transcribing_short}"
                                                    }
                                                } else if file_exists {
                                                    button {
                                                        class: "text-xs text-ios-orange-dark px-1 pb-1 active:opacity-70 hover:opacity-70 transition-opacity duration-150",
                                                        onclick: move |_| {
                                                            let aid = audio_id_tr.clone();
                                                            let fp = file_path_tr.clone();
                                                            let path = audio::resolve_audio_path(&fp);
                                                            transcribe_error.set(None);
                                                            transcribing_audio_id.set(Some(aid.clone()));
                                                            spawn(async move {
                                                                let result = retranscribe_audio(
                                                                    &db(),
                                                                    &aid,
                                                                    std::path::Path::new(&path),
                                                                ).await;
                                                                if let Err(e) = result {
                                                                    eprintln!("[transcribe] error: {e}");
                                                                    transcribe_error.set(Some((aid.clone(), e)));
                                                                }
                                                                transcribing_audio_id.set(None);
                                                                audios_version.set(audios_version() + 1);
                                                            });
                                                        },
                                                        "{transcribe_action}"
                                                    }
                                                }
                                                if let Some(ref msg) = error_msg {
                                                    p { class: "text-xs text-ios-red px-1 pb-1", "{msg}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AudioJobBanner(local_note_id: Signal<String>) -> Element {
    let app: AppState = use_context();
    let manager: TranscriptionManager = use_context();
    let lang = (app.current_lang)();

    let audio_job_status: Option<JobStatus> = {
        let jobs = (app.transcription_jobs)();
        let nid = local_note_id();
        if nid.is_empty() {
            None
        } else {
            jobs.get(&nid)
                .and_then(|q| q.front())
                .map(|j| j.status.clone())
        }
    };

    rsx! {
        if let Some(ref js) = audio_job_status {
            match js {
                JobStatus::Failed(reason) => {
                    let failed_label = t(&lang, "audio-transcription-failed");
                    let retry_label = t(&lang, "audio-import-retry");
                    let msg = format!("{failed_label}: {reason}");
                    let retry_manager = manager.clone();
                    let dismiss_manager = manager.clone();
                    rsx! {
                        div { class: "flex items-center gap-2 px-3 py-2 mt-2 bg-ios-red/10 rounded-lg",
                            span { class: "text-xs text-stone-600 flex-1", "{msg}" }
                            button {
                                class: "text-xs text-ios-orange-dark font-medium active:opacity-70",
                                onclick: move |_| {
                                    let nid = local_note_id();
                                    retry_manager.retry(&nid);
                                },
                                "{retry_label}"
                            }
                            button {
                                class: "text-stone-400 active:opacity-70",
                                onclick: move |_| {
                                    let nid = local_note_id();
                                    dismiss_manager.dismiss(&nid);
                                },
                                IconX { size: 14 }
                            }
                        }
                    }
                }
                JobStatus::Done(_) => rsx! {},
                other => {
                    let transcribing_label = t(&lang, "audio-transcribing");
                    let label = match other {
                        JobStatus::Polling { elapsed_s } => {
                            format!("{transcribing_label} · {}:{:02}", elapsed_s / 60, elapsed_s % 60)
                        }
                        _ => transcribing_label,
                    };
                    rsx! {
                        div { class: "flex items-center gap-2 px-3 py-2 mt-2 bg-ios-orange/10 rounded-lg",
                            span { class: "inline-block w-3 h-3 border-2 border-ios-orange border-t-transparent rounded-full animate-spin" }
                            span { class: "text-xs text-stone-600", "{label}" }
                        }
                    }
                }
            }
        }
    }
}
