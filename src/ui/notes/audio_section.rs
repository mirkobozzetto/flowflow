use crate::application::i18n::{t, t_args};
use crate::application::transcribe_audio::retranscribe_audio;
use crate::application::transcription_manager::{
    JobStatus, TranscriptionManager,
};
use crate::domain::NoteAudio;
use crate::infrastructure::audio::{self, RecordingState};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::IconX;
use crate::ui::notes::audio_player::AudioPlayer;
use crate::ui::notes::dates::format_relative_date;
use crate::ui::notes::playback::{NotePlayer, Playback};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn AudioSection(
    local_note_id: Signal<String>,
    mut audios_version: Signal<u32>,
    mut player: NotePlayer,
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

    let transcribing_short = t(&lang, "note-transcribing-short");
    let transcribe_action = t(&lang, "note-transcribe-action");

    rsx! {
        if !audios.is_empty() {
            {
                let count = audios.len().to_string();
                let audio_label = t_args(&lang, "note-audios-label", &[("count", &count)]);
                rsx! {
                    div { class: "border-t border-stone-200/70 mt-4 pt-3",
                        button {
                            class: "pressable flex items-center gap-2 w-full min-h-[44px] text-left",
                            onclick: move |_| audios_expanded.set(!audios_expanded()),
                            span { class: "text-xs font-medium text-stone-400 uppercase tracking-[0.08em]", "{audio_label}" }
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
                                        let transcribed = audio.transcription.is_some();
                                        let is_playing_this = player.is_playing(&audio.id);
                                        let audio_id_play = audio.id.clone();
                                        let path_play = resolved.clone();
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
                                                        // Read on tap: the words only drive the
                                                        // highlight, which lives in the note body.
                                                        let transcript = db().words_for_audio(&audio_id_play).unwrap_or_default();
                                                        player.toggle(Playback {
                                                            audio_id: audio_id_play.clone(),
                                                            path: path_play.clone(),
                                                            transcript,
                                                            duration_secs: duration_play,
                                                        }, 0.0);
                                                    },
                                                    on_stop: move |_| player.pause(),
                                                    on_delete: move |_| {
                                                        if player.is_current(&audio_id) {
                                                            player.stop();
                                                        }
                                                        let fp = file_path.clone();
                                                        let _ = std::fs::remove_file(audio::resolve_audio_path(&fp));
                                                        let _ = db().delete_audio(&audio_id);
                                                        audios_version.set(audios_version() + 1);
                                                        app.notes_version.set((app.notes_version)() + 1);
                                                    },
                                                }
                                                if !transcribed && is_transcribing_this {
                                                    p {
                                                        class: "text-xs text-stone-400 px-1 pb-1",
                                                        style: "animation: pulseSoft 1.5s ease-in-out infinite;",
                                                        "{transcribing_short}"
                                                    }
                                                } else if !transcribed && file_exists {
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

    // While the voice capsule follows this note's job (#183), it is the
    // progress and failure UI; the banner is the fallback once it is closed.
    let capsule_busy = (app.recording_state)() == RecordingState::Transcribing;
    let audio_job_status: Option<JobStatus> = {
        let jobs = (app.transcription_jobs)();
        let nid = local_note_id();
        if nid.is_empty() || capsule_busy {
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
                        JobStatus::Polling { percent: Some(p), .. } => {
                            t_args(&lang, "audio-transcribing-percent", &[("percent", &p.to_string())])
                        }
                        JobStatus::Polling { elapsed_s, percent: None } => {
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
