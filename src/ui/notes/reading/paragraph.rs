use crate::domain::Word;
use dioxus::prelude::*;
use std::rc::Rc;

/// One spoken paragraph, a span per word, each seeking to its own start.
///
/// `active` is relative to this paragraph, so a moving highlight re-renders
/// the one or two paragraphs it crosses, never the whole note. Desktop renders
/// the same spans inert: `afplay` exposes no position, so nothing is seekable.
#[component]
pub fn TimedParagraph(
    words: Rc<[Word]>,
    active: Option<usize>,
    seekable: bool,
    on_seek: EventHandler<f64>,
) -> Element {
    rsx! {
        p { class: "mb-3 text-[15px] leading-relaxed text-stone-800 break-words",
            for (index, word) in words.iter().enumerate() {
                {
                    let start_secs = f64::from(word.start_ms) / 1000.0;
                    rsx! {
                        span {
                            key: "{index}",
                            class: if active == Some(index) {
                                "rounded bg-ios-orange/20 py-0.5"
                            } else {
                                "py-0.5"
                            },
                            class: if seekable { "active:opacity-60 transition-opacity duration-150" },
                            onclick: move |evt| {
                                if seekable {
                                    evt.stop_propagation();
                                    on_seek.call(start_secs);
                                }
                            },
                            "{word.text} "
                        }
                    }
                }
            }
        }
    }
}
