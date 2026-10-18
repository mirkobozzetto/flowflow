use crate::application::chapters::generate_chapters;
use crate::domain::reading::{
    locate_paragraphs, Chapter, Segment, PAUSE_PARAGRAPH_MS,
};
use crate::domain::Word;
use crate::infrastructure::audio;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::device_log;
use crate::ui::chat::md_to_html;
use crate::ui::notes::playback::{NotePlayer, Playback};
use dioxus::prelude::*;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

mod bar;
mod heading;
mod paragraph;
mod sheet;
use bar::ReadingBar;
use heading::ChapterHeading;
use paragraph::TimedParagraph;

const TEXT_BOX: &str = "w-full min-h-[300px] p-3 mt-3 rounded-xl border border-stone-300/60 bg-transparent shadow-none text-sm prose prose-sm max-w-none text-stone-900 break-words overflow-x-hidden [&_*]:[overflow-wrap:anywhere] [&_pre]:whitespace-pre-wrap [&_pre]:break-words";
const TEXT_SEGMENT: &str = "prose mb-3 text-[15px] leading-relaxed text-stone-800 break-words overflow-x-hidden [&_*]:[overflow-wrap:anywhere] [&_pre]:whitespace-pre-wrap [&_pre]:break-words";

/// A recording whose words carry timings, playable from any of them.
#[derive(Clone, PartialEq)]
struct TimedClip {
    playback: Playback,
    seekable: bool,
    chapters: Option<Vec<Chapter>>,
}

/// A chapter as the note shows it, across all its recordings.
#[derive(Clone, PartialEq)]
struct ChapterMark {
    chapter: Chapter,
    clip: usize,
    start_ms: u32,
    /// Its rank in the note, the number the scroll script reports.
    n: usize,
}

#[derive(Clone, PartialEq)]
enum Piece {
    Text(String),
    Paragraph {
        clip: usize,
        first: usize,
        words: Rc<[Word]>,
        /// The chapter this paragraph opens.
        chapter: Option<ChapterMark>,
    },
}

/// A time in the recording, `m:ss`; past an hour the minutes keep counting.
fn clock(ms: u32) -> String {
    let secs = ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
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
            let chapters = db.chapters_for_audio(&clip.id);
            Some(TimedClip {
                playback: Playback {
                    audio_id: clip.id,
                    path,
                    transcript,
                    duration_secs: clip.duration_secs.unwrap_or(0.0),
                },
                seekable,
                chapters,
            })
        })
        .collect()
}

fn pieces(content: &str, clips: &[TimedClip]) -> Vec<Piece> {
    let transcripts: Vec<&[Word]> = clips
        .iter()
        .map(|c| c.playback.transcript.words.as_slice())
        .collect();
    let mut next_chapter = vec![0; clips.len()];
    let mut marked = 0;
    locate_paragraphs(content, &transcripts, PAUSE_PARAGRAPH_MS)
        .into_iter()
        .map(|segment| match segment {
            Segment::Text(text) => Piece::Text(text),
            Segment::Paragraph { audio, words } => {
                // The last chapter opened by now: when the paragraph opening
                // a chapter was edited by hand, its heading moves down to the
                // next timed one instead of disappearing.
                let chapters =
                    clips[audio].chapters.as_deref().unwrap_or_default();
                let mut chapter = None;
                while let Some(c) = chapters
                    .get(next_chapter[audio])
                    .filter(|c| c.start_word <= words.start)
                {
                    chapter = transcripts[audio].get(c.start_word).map(|w| {
                        ChapterMark {
                            chapter: c.clone(),
                            clip: audio,
                            start_ms: w.start_ms,
                            n: marked,
                        }
                    });
                    next_chapter[audio] += 1;
                }
                // Two chapters on one paragraph keep the later one.
                if chapter.is_some() {
                    marked += 1;
                }
                Piece::Paragraph {
                    clip: audio,
                    first: words.start,
                    words: transcripts[audio][words].into(),
                    chapter,
                }
            }
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
    let mut chapters_version = use_signal(|| 0u32);
    let clips = use_memo(move || {
        let id = local_note_id();
        let _ = audios_version();
        let _ = chapters_version();
        if id.is_empty() {
            Vec::new()
        } else {
            timed_clips(&db(), &id)
        }
    });
    let pieces = use_memo(move || pieces(&content(), &clips.read()));

    // Chapters come on their own: right after a transcription, or when a
    // note transcribed before is opened. One try per transcript and opening,
    // so a refusal (no AI consent, offline) leaves plain paragraphs.
    let mut tried = use_signal(HashSet::<(String, usize)>::new);
    use_effect(move || {
        for clip in clips.read().iter().filter(|c| c.chapters.is_none()) {
            let audio_id = clip.playback.audio_id.clone();
            let words = &clip.playback.transcript.words;
            if !tried.write().insert((audio_id.clone(), words.len())) {
                continue;
            }
            let words = words.clone();
            spawn(async move {
                let db = db();
                match generate_chapters(&db, &audio_id, &words).await {
                    Ok(chapters) => {
                        device_log(&format!(
                            "[chapters] {audio_id}: {} chapters",
                            chapters.len()
                        ));
                        chapters_version.set(chapters_version() + 1);
                    }
                    Err(e) => {
                        device_log(&format!("[chapters] {audio_id}: {e}"))
                    }
                }
            });
        }
    });

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
    let marks: Rc<[ChapterMark]> = pieces
        .read()
        .iter()
        .filter_map(|p| match p {
            Piece::Paragraph { chapter, .. } => chapter.clone(),
            Piece::Text(_) => None,
        })
        .collect();
    let with_bar = any_seekable || !marks.is_empty();

    rsx! {
        if with_bar {
            ReadingBar { marks, clips, player }
        }
        div {
            class: if with_bar { "mt-2" } else { "mt-4" },
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
                    Piece::Paragraph { clip, first, words, chapter } => {
                        let active = active_word
                            .filter(|_| playing == Some(clip))
                            .and_then(|w| w.checked_sub(first))
                            .filter(|&w| w < words.len());
                        let seekable = clips.read()[clip].seekable;
                        let mut player = player;
                        rsx! {
                            div { key: "{clip}-{first}",
                                if let Some(mark) = chapter {
                                    ChapterHeading { mark, first: i == 0 }
                                }
                                TimedParagraph {
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
}
