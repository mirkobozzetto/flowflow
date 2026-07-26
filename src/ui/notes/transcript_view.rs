use crate::domain::Word;
use dioxus::prelude::*;

/// The transcript rendered one span per word, each seeking to its own start.
///
/// Desktop renders the same spans inert: the word data ships on both platforms,
/// only the player differs (`afplay` exposes no position), so the parent simply
/// passes no handler there.
#[component]
pub fn TranscriptView(
    words: Vec<Word>,
    active: Option<usize>,
    seekable: bool,
    on_seek: EventHandler<f64>,
) -> Element {
    rsx! {
        p { class: "text-xs text-stone-600 italic px-1 pb-1 leading-relaxed",
            for (index, word) in words.into_iter().enumerate() {
                span {
                    key: "{index}",
                    class: if active == Some(index) {
                        "rounded bg-ios-orange/20 py-0.5"
                    } else {
                        "py-0.5"
                    },
                    class: if seekable { "active:opacity-60 transition-opacity duration-150" },
                    onclick: move |_| {
                        if seekable {
                            on_seek.call(f64::from(word.start_ms) / 1000.0);
                        }
                    },
                    "{word.text} "
                }
            }
        }
    }
}
