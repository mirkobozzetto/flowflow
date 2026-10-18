use crate::domain::reading::Chapter;
use dioxus::prelude::*;

/// The heading over a chapter: where it starts in the recording, its title,
/// and one grey line on what it covers.
#[component]
pub fn ChapterHeading(chapter: Chapter, start_ms: u32, first: bool) -> Element {
    let secs = start_ms / 1000;
    let clock = format!("{}:{:02}", secs / 60, secs % 60);
    rsx! {
        div {
            class: if first { "mt-1 mb-3" } else { "mt-8 pt-5 mb-3 border-t border-stone-200/80" },
            p { class: "text-xs tabular-nums font-medium text-ios-orange-dark", "{clock}" }
            h3 { class: "mt-0.5 text-[17px] font-semibold tracking-[-0.01em] text-stone-900",
                "{chapter.title}"
            }
            if !chapter.summary.is_empty() {
                p { class: "mt-1 text-sm leading-snug text-stone-500", "{chapter.summary}" }
            }
        }
    }
}
