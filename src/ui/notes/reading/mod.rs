use crate::domain::reading::{locate_paragraphs, Segment, PAUSE_PARAGRAPH_MS};
use crate::domain::Word;
use crate::infrastructure::audio;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::md_to_html;
use crate::ui::notes::playback::{NotePlayer, Playback};
use dioxus::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

mod paragraph;
use paragraph::TimedParagraph;

const TEXT_BOX: &str = "w-full min-h-[300px] p-3 mt-3 rounded-xl border border-stone-300/60 bg-transparent shadow-none text-sm prose prose-sm max-w-none text-stone-900 break-words overflow-x-hidden [&_*]:[overflow-wrap:anywhere] [&_pre]:whitespace-pre-wrap [&_pre]:break-words";
const TEXT_SEGMENT: &str = "prose mb-3 text-[15px] leading-relaxed text-stone-800 break-words overflow-x-hidden [&_*]:[overflow-wrap:anywhere] [&_pre]:whitespace-pre-wrap [&_pre]:break-words";

/// A recording whose words carry timings, playable from any of them.
#[derive(Clone, PartialEq)]
struct TimedClip {
    playback: Playback,
    seekable: bool,
}

#[derive(Clone, PartialEq)]
enum Piece {
    Text(String),
    Paragraph {
        clip: usize,
        first: usize,
        words: Rc<[Word]>,
    },
}

fn timed_clips(db: &Database, note_id: &str) -> Vec<TimedClip> {
    db.list_audios(note_id)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|clip| {
            let transcript = db.words_for_audio(&clip.id)?;
            let path = audio::resolve_audio_path(&clip.file_path);
            let seekable =
                cfg!(target_os = "ios") && std::path::Path::new(&path).exists();
            Some(TimedClip {
                playback: Playback {
                    audio_id: clip.id,
                    path,
                    transcript,
                    duration_secs: clip.duration_secs.unwrap_or(0.0),
                },
                seekable,
            })
        })
        .collect()
}

fn pieces(content: &str, clips: &[TimedClip]) -> Vec<Piece> {
    let transcripts: Vec<&[Word]> = clips
        .iter()
        .map(|c| c.playback.transcript.words.as_slice())
        .collect();
    locate_paragraphs(content, &transcripts, PAUSE_PARAGRAPH_MS)
        .into_iter()
        .map(|segment| match segment {
            Segment::Text(text) => Piece::Text(text),
            Segment::Paragraph { audio, words } => Piece::Paragraph {
                clip: audio,
                first: words.start,
                words: transcripts[audio][words].into(),
            },
        })
        .collect()
}

/// The note body out of edit mode: spoken paragraphs with tappable words when
/// the note holds a timed recording, the plain text box otherwise.
#[component]
pub fn ReadingView(
    content: Signal<String>,
    local_note_id: Signal<String>,
    audios_version: Signal<u32>,
    player: NotePlayer,
    placeholder: String,
    on_edit: EventHandler<()>,
) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let clips = use_memo(move || {
        let id = local_note_id();
        let _ = audios_version();
        if id.is_empty() {
            Vec::new()
        } else {
            timed_clips(&db(), &id)
        }
    });
    let pieces = use_memo(move || pieces(&content(), &clips.read()));

    let has_timed = pieces
        .read()
        .iter()
        .any(|p| matches!(p, Piece::Paragraph { .. }));
    if !has_timed {
        let body = content();
        return rsx! {
            div { class: TEXT_BOX, onclick: move |_| on_edit.call(()),
                if body.trim().is_empty() {
                    span { class: "text-stone-400", "{placeholder}" }
                } else {
                    div { dangerous_inner_html: md_to_html(&body) }
                }
            }
        };
    }

    let playing = player.playback.read().as_ref().and_then(|p| {
        clips
            .read()
            .iter()
            .position(|c| c.playback.audio_id == p.audio_id)
    });
    let active_word = (player.active_word)();
    // Without a playable clip (desktop, audio file gone) a tap edits, as the
    // plain text box does.
    let any_seekable = clips.read().iter().any(|c| c.seekable);

    rsx! {
        div {
            class: "mt-4",
            onclick: move |_| {
                if !any_seekable {
                    on_edit.call(());
                }
            },
            for (i, piece) in pieces.read().iter().enumerate() {
                match piece.clone() {
                    Piece::Text(text) => rsx! {
                        div {
                            key: "t{i}",
                            class: TEXT_SEGMENT,
                            onclick: move |_| on_edit.call(()),
                            dangerous_inner_html: md_to_html(&text),
                        }
                    },
                    Piece::Paragraph { clip, first, words } => {
                        let active = active_word
                            .filter(|_| playing == Some(clip))
                            .and_then(|w| w.checked_sub(first))
                            .filter(|&w| w < words.len());
                        let seekable = clips.read()[clip].seekable;
                        let mut player = player;
                        rsx! {
                            TimedParagraph {
                                key: "{clip}-{first}",
                                words,
                                active,
                                seekable,
                                on_seek: move |secs: f64| {
                                    let playback = clips.read()[clip].playback.clone();
                                    player.play(playback, secs);
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}
