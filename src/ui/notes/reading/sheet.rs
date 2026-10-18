use super::{clock, ChapterMark};
use crate::application::i18n::{t, t_args};
use crate::infrastructure::platform::haptic;
use crate::ui::hooks::swipe::use_sheet_dismiss;
use crate::ui::icons::{IconArrowUp, IconX};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::rc::Rc;
use std::time::Duration;

const SHEET_ID: &str = "chapter-sheet";
const BACKDROP_ID: &str = "chapter-backdrop";
/// Shared with the scroll script, which centers this list on open.
pub const LIST_ID: &str = "chapter-list";
/// The grabber and the title: a drag starting there pulls the sheet down.
const GRAB_PX: f64 = 56.0;
const DISMISS_AT: f64 = 0.3;
/// Matches `chapter-sheet-out` in tailwind.css.
const CLOSE_MS: u64 = 300;
/// Rows past this one appear together, so a long list never trails in.
const STAGGER_ROWS: usize = 12;

/// The sheet the reading bar opens: the note's top, then every chapter, the
/// one under the bar in orange and centered. `on_pick(None)` is the top of
/// the note; the sheet slides down, then `on_close` unmounts it.
#[component]
pub fn ChapterSheet(
    marks: Rc<[ChapterMark]>,
    current: usize,
    /// The chapter the audio is in, and whether it plays or is paused.
    live: Option<usize>,
    playing: bool,
    on_open: EventHandler<()>,
    on_pick: EventHandler<Option<usize>>,
    on_close: EventHandler<()>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut closing = use_signal(|| false);

    use_sheet_dismiss(SHEET_ID, BACKDROP_ID, GRAB_PX, DISMISS_AT, move || {
        haptic("soft");
        on_close.call(());
    });
    use_effect(move || on_open.call(()));

    let mut close = move |target: Option<Option<usize>>| {
        if closing() {
            return;
        }
        closing.set(true);
        match target {
            Some(target) => {
                haptic("light");
                on_pick.call(target);
            }
            None => haptic("soft"),
        }
        spawn(async move {
            futures_timer::Delay::new(Duration::from_millis(CLOSE_MS)).await;
            on_close.call(());
        });
    };

    rsx! {
        div {
            id: BACKDROP_ID,
            class: "chapter-backdrop fixed inset-0 z-40 bg-stone-900/20",
            "data-closing": closing().then_some("true"),
            onclick: move |_| close(None),
        }
        div {
            id: SHEET_ID,
            class: "chapter-sheet glass-panel fixed left-2 right-2 z-50 rounded-[38px] px-5 pt-2 pb-[22px]",
            style: "bottom: calc(env(safe-area-inset-bottom) + 8px);",
            "data-closing": closing().then_some("true"),
            role: "dialog",
            div { class: "w-9 h-[5px] mx-auto mb-2.5 rounded-full bg-stone-300" }
            div { class: "flex items-center gap-2.5 mb-2",
                span { class: "text-[15px] font-semibold text-stone-900",
                    {t(&lang, "reading-chapters")}
                }
                button {
                    class: "pressable ml-auto glass-disc relative w-9 h-9 -mr-1 flex items-center justify-center rounded-full text-stone-500",
                    aria_label: t(&lang, "reading-close"),
                    onclick: move |_| close(None),
                    IconX { size: 16 }
                }
            }
            div {
                id: LIST_ID,
                class: "max-h-[min(58vh,440px)] overflow-y-auto overscroll-contain -mx-2",
                button {
                    class: "chapter-row pressable w-full flex items-center gap-3 min-h-[48px] px-3 rounded-2xl text-left active:bg-stone-900/5",
                    onclick: move |_| close(Some(None)),
                    span { class: "w-12 shrink-0 flex text-stone-400", IconArrowUp { size: 14 } }
                    span { class: "flex-1 min-w-0 truncate text-[15px] text-stone-900",
                        {t(&lang, "reading-note-start")}
                    }
                }
                for mark in marks.iter() {
                    {
                        let n = mark.n;
                        let on = n == current;
                        let minutes = ((mark.len_ms + 30_000) / 60_000).max(1).to_string();
                        rsx! {
                            button {
                                key: "{n}",
                                class: if on {
                                    "chapter-row pressable w-full flex items-start gap-3 min-h-[48px] py-2.5 px-3 rounded-2xl text-left bg-ios-orange/10"
                                } else {
                                    "chapter-row pressable w-full flex items-start gap-3 min-h-[48px] py-2.5 px-3 rounded-2xl text-left active:bg-stone-900/5"
                                },
                                style: "--i: {(n + 1).min(STAGGER_ROWS)}",
                                "data-current": on.then_some("true"),
                                onclick: move |_| close(Some(Some(n))),
                                span {
                                    class: if on {
                                        "w-12 shrink-0 pt-0.5 text-xs tabular-nums font-medium text-ios-orange-dark"
                                    } else {
                                        "w-12 shrink-0 pt-0.5 text-xs tabular-nums font-medium text-stone-400"
                                    },
                                    "{clock(mark.start_ms)}"
                                }
                                span { class: "flex-1 min-w-0",
                                    span {
                                        class: if on {
                                            "block truncate text-[15px] font-semibold text-ios-orange-dark"
                                        } else {
                                            "block truncate text-[15px] text-stone-900"
                                        },
                                        "{mark.chapter.title}"
                                    }
                                    if !mark.chapter.summary.is_empty() {
                                        span { class: "block mt-0.5 truncate text-xs text-stone-500",
                                            "{mark.chapter.summary}"
                                        }
                                    }
                                }
                                if live == Some(n) {
                                    span {
                                        class: "chapter-wave shrink-0 mt-1",
                                        "data-playing": playing.then_some("true"),
                                        i {}
                                        i {}
                                        i {}
                                    }
                                }
                                span { class: "shrink-0 pt-0.5 text-xs tabular-nums text-stone-400",
                                    {t_args(&lang, "reading-minutes", &[("n", &minutes)])}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
