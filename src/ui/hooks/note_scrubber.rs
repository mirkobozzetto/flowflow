use crate::infrastructure::platform::haptic;
use dioxus::prelude::*;

const NOTE_SCRUBBER_JS: &str = include_str!("note_scrubber.js");

/// The bubble's words, in the app's language.
pub struct ScrubberLabels<'a> {
    pub chapter: &'a str,
    pub start: &'a str,
    pub end: &'a str,
}

// Kept apart from the hook (which needs a Dioxus runtime) so it is testable.
pub fn build_note_scrubber(
    root_id: &str,
    bar_id: &str,
    labels: &ScrubberLabels,
) -> String {
    let lit = |s: &str| serde_json::to_string(s).unwrap_or_default();
    NOTE_SCRUBBER_JS
        .replace("\"__ROOT__\"", &lit(root_id))
        .replace("\"__BAR__\"", &lit(bar_id))
        .replace("\"__CHAPTER__\"", &lit(labels.chapter))
        .replace("\"__START__\"", &lit(labels.start))
        .replace("\"__END__\"", &lit(labels.end))
}

/// Runs a script built by `build_note_scrubber`; a grab taps.
pub fn use_note_scrubber(script: String) {
    use_future(move || {
        let script = script.clone();
        async move {
            let mut eval = document::eval(&script);
            while let Ok(msg) = eval.recv::<String>().await {
                if msg == "grab" {
                    haptic("light");
                }
            }
        }
    });
}
