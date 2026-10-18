use super::bar::BAR_ID;
use super::ChapterMark;
use crate::application::i18n::t;
use crate::ui::hooks::note_scrubber::{
    build_note_scrubber, use_note_scrubber, ScrubberLabels,
};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::rc::Rc;

/// Shared with the scrubber script.
const ROOT_ID: &str = "note-scrubber";

/// The glass knob on the right edge of a long note. Held, it scrubs through
/// the note on a track marked at each chapter (styles: tailwind.css).
#[component]
pub fn NoteScrubber(marks: Rc<[ChapterMark]>) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let script = use_hook(|| {
        build_note_scrubber(
            ROOT_ID,
            BAR_ID,
            &ScrubberLabels {
                chapter: &t(&lang, "reading-scrub-chapter"),
                start: &t(&lang, "reading-note-start"),
                end: &t(&lang, "reading-note-end"),
            },
        )
    });
    use_note_scrubber(script);

    rsx! {
        div { id: ROOT_ID, class: "note-scrubber", aria_hidden: "true",
            div { class: "note-scrubber-track",
                for mark in marks.iter() {
                    span {
                        key: "{mark.n}",
                        class: "note-scrubber-tick",
                        "data-tick": "{mark.n}",
                    }
                }
            }
            div { class: "note-scrubber-knob", "data-scrub": "knob",
                i {}
                i {}
            }
            div { class: "note-scrubber-bubble", "data-scrub": "bubble",
                div { class: "flex items-center gap-2",
                    span {
                        class: "text-[11px] font-semibold tabular-nums text-ios-orange-dark",
                        "data-scrub": "n",
                    }
                    span {
                        class: "text-[11px] tabular-nums text-stone-400",
                        "data-scrub": "clock",
                    }
                }
                p {
                    class: "mt-0.5 text-[15px] font-semibold leading-snug text-stone-900 line-clamp-2",
                    "data-scrub": "title",
                }
            }
        }
    }
}
