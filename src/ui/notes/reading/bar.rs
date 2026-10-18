use super::sheet::ChapterSheet;
use super::{clock, ChapterMark, TimedClip};
use crate::application::i18n::t;
use crate::ui::hooks::chapter_scroll::use_chapter_scroll;
use crate::ui::notes::playback::NotePlayer;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::rc::Rc;

/// Shared with the scroll script, which measures the bar to know which
/// chapter heading sits under it.
const BAR_ID: &str = "reading-bar";

/// The pill stuck to the top of the note: play and pause from anywhere, the
/// time, and the chapter under it, which opens the chapter sheet.
///
/// Rendered as a direct child of the note's scroller, so it stays stuck down
/// to the end of the note, not only over the text.
#[component]
pub fn ReadingBar(
    marks: Rc<[ChapterMark]>,
    clips: Memo<Vec<TimedClip>>,
    player: NotePlayer,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let scroll = use_chapter_scroll(BAR_ID);
    let mut sheet_open = use_signal(|| false);

    let n = (scroll.current)().min(marks.len().saturating_sub(1));
    let current = marks.get(n).cloned();

    // Before anything plays, play starts at the chapter in view.
    let start = {
        let clips = clips.read();
        current
            .as_ref()
            .filter(|m| clips[m.clip].seekable)
            .map(|m| (m.clip, f64::from(m.start_ms) / 1000.0))
            .or_else(|| clips.iter().position(|c| c.seekable).map(|i| (i, 0.0)))
    };

    let paused_at = (player.paused_at)();
    let active_word = (player.active_word)();
    let (loaded, time_ms) = match player.playback.read().as_ref() {
        Some(p) => (
            true,
            paused_at.map(|s| (s * 1000.0) as u32).unwrap_or_else(|| {
                active_word
                    .and_then(|i| p.transcript.words.get(i))
                    .map_or(0, |w| w.start_ms)
            }),
        ),
        None => (false, current.as_ref().map_or(0, |m| m.start_ms)),
    };
    let playing = loaded && paused_at.is_none();

    rsx! {
        div {
            id: BAR_ID,
            // -0.75rem cancels the scroller's pt-3, so the bar sticks flush
            // and no text shows above it.
            class: "sticky z-10 -mx-4 px-4 pt-3 pb-2 bg-stone-100",
            style: "top: -0.75rem;",
            "data-idle": if loaded { "false" } else { "true" },
            div { class: "w-full h-11 flex items-center gap-1 pl-1.5 pr-1 rounded-full bg-warm-white border border-stone-200 shadow-card",
                if loaded || start.is_some() {
                    button {
                        class: "pressable w-8 h-8 shrink-0 rounded-full bg-ios-orange/80 text-white flex items-center justify-center",
                        aria_label: if playing { t(&lang, "reading-pause") } else { t(&lang, "reading-play") },
                        onclick: move |_| {
                            let mut player = player;
                            let loaded = player.playback.peek().clone();
                            if let Some(clip) = loaded {
                                player.toggle(clip, 0.0);
                            } else if let Some((i, secs)) = start {
                                let clip = clips.read()[i].playback.clone();
                                player.play(clip, secs);
                            }
                        },
                        if playing {
                            svg {
                                width: "14", height: "14", view_box: "0 0 14 14", fill: "currentColor",
                                rect { x: "1", y: "1", width: "4", height: "12", rx: "1" }
                                rect { x: "9", y: "1", width: "4", height: "12", rx: "1" }
                            }
                        } else {
                            svg {
                                width: "14", height: "14", view_box: "0 0 14 14", fill: "currentColor",
                                polygon { points: "2,0 14,7 2,14" }
                            }
                        }
                    }
                }
                button {
                    class: "pressable flex-1 min-w-0 h-full flex items-center gap-2 pl-1 pr-2 rounded-full text-left",
                    disabled: marks.is_empty(),
                    onclick: move |_| sheet_open.set(true),
                    span {
                        class: "w-10 shrink-0 text-xs tabular-nums font-medium text-ios-orange-dark",
                        "data-bar": "clock",
                        "{clock(time_ms)}"
                    }
                    if let Some(mark) = current {
                        span {
                            class: "flex-1 min-w-0 truncate text-sm font-medium text-stone-900",
                            "data-bar": "title",
                            "{mark.chapter.title}"
                        }
                        span {
                            class: "text-xs tabular-nums text-stone-400",
                            "data-bar": "count",
                            "{n + 1}/{marks.len()}"
                        }
                        span { class: "inline-block w-1.5 h-1.5 ml-1 -mt-1 border-r-[1.5px] border-b-[1.5px] border-stone-400 rotate-45" }
                    }
                }
            }
        }
        if sheet_open() {
            ChapterSheet {
                marks: marks.clone(),
                current: n,
                on_pick: move |target: Option<usize>| {
                    sheet_open.set(false);
                    scroll.jump(target);
                },
                on_close: move |_| sheet_open.set(false),
            }
        }
    }
}
