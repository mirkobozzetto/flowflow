use super::{clock, ChapterMark};
use dioxus::prelude::*;

/// The heading over a chapter: where it starts in the recording, its title,
/// and one grey line on what it covers.
#[component]
pub fn ChapterHeading(mark: ChapterMark, first: bool) -> Element {
    let chapter = mark.chapter;
    rsx! {
        div {
            "data-chapter": "{mark.n}",
            class: if first { "mt-1 mb-3" } else { "mt-8 pt-5 mb-3 border-t border-stone-200/80" },
            p {
                class: "text-xs tabular-nums font-medium text-ios-orange-dark",
                "data-heading": "clock",
                "{clock(mark.start_ms)}"
            }
            h3 {
                class: "mt-0.5 text-[17px] font-semibold tracking-[-0.01em] text-stone-900",
                "data-heading": "title",
                "{chapter.title}"
            }
            if !chapter.summary.is_empty() {
                p { class: "mt-1 text-sm leading-snug text-stone-500", "{chapter.summary}" }
            }
        }
    }
}
