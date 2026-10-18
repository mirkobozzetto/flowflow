use super::{clock, ChapterMark};
use crate::application::i18n::t;
use crate::ui::icons::IconX;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::rc::Rc;

/// The sheet the reading bar opens: the note's top, then every chapter, the
/// one under the bar in orange. `on_pick(None)` is the top of the note.
#[component]
pub fn ChapterSheet(
    marks: Rc<[ChapterMark]>,
    current: usize,
    on_pick: EventHandler<Option<usize>>,
    on_close: EventHandler<()>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    rsx! {
        div {
            class: "fixed inset-0 z-40 bg-stone-900/20 backdrop-fade",
            onclick: move |_| on_close.call(()),
        }
        div {
            class: "glass-panel fixed left-2 right-2 z-50 rounded-[38px] px-5 pt-[18px] pb-[22px]",
            style: "bottom: calc(env(safe-area-inset-bottom) + 8px); animation: fadeInUp 260ms var(--ease-soft) both;",
            role: "dialog",
            div { class: "flex items-center gap-2.5 mb-2",
                span { class: "text-[15px] font-semibold text-stone-900",
                    {t(&lang, "reading-chapters")}
                }
                button {
                    class: "pressable ml-auto glass-disc relative w-9 h-9 -mr-1 flex items-center justify-center rounded-full text-stone-500",
                    aria_label: t(&lang, "reading-close"),
                    onclick: move |_| on_close.call(()),
                    IconX { size: 16 }
                }
            }
            div { class: "max-h-[360px] overflow-y-auto -mx-2",
                SheetRow {
                    lead: "↑".to_string(),
                    title: t(&lang, "reading-note-start"),
                    on: false,
                    onclick: move |_| on_pick.call(None),
                }
                for mark in marks.iter() {
                    {
                        let n = mark.n;
                        rsx! {
                            SheetRow {
                                key: "{n}",
                                lead: clock(mark.start_ms),
                                title: mark.chapter.title.clone(),
                                on: n == current,
                                onclick: move |_| on_pick.call(Some(n)),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SheetRow(
    lead: String,
    title: String,
    on: bool,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            class: if on {
                "w-full flex items-center gap-3 min-h-[48px] px-3 rounded-2xl text-left bg-ios-orange/10"
            } else {
                "w-full flex items-center gap-3 min-h-[48px] px-3 rounded-2xl text-left active:bg-stone-900/5"
            },
            onclick: move |_| onclick.call(()),
            span {
                class: if on {
                    "w-12 shrink-0 text-xs tabular-nums font-medium text-ios-orange-dark"
                } else {
                    "w-12 shrink-0 text-xs tabular-nums font-medium text-stone-400"
                },
                "{lead}"
            }
            span {
                class: if on {
                    "flex-1 min-w-0 truncate text-[15px] font-semibold text-ios-orange-dark"
                } else {
                    "flex-1 min-w-0 truncate text-[15px] text-stone-900"
                },
                "{title}"
            }
        }
    }
}
