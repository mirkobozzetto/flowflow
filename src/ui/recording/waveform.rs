use crate::ui::AppState;
use dioxus::prelude::*;

// Timeline of the voice capsule, Voice Memos style: `app.audio_levels` is the
// history of per-slice levels (one per 50 ms, newest last). A dotted track
// fills the left, bars grow from the right, the left edge is masked so old
// bars fade out. No smoothing or per-bar animation: each slice is honest.
const TRACK: &str = "-webkit-mask-image: linear-gradient(90deg, transparent, #000 12px, #000 100%); mask-image: linear-gradient(90deg, transparent, #000 12px, #000 100%);";
const DOTS: &str = "background: radial-gradient(circle, rgba(255,255,255,0.45) 1.5px, transparent 1.6px) 0 50% / 7px 3px repeat-x;";

#[component]
pub fn Waveform() -> Element {
    let app: AppState = use_context();
    let bars = (app.audio_levels)();

    rsx! {
        div {
            class: "flex-1 flex items-center justify-end gap-[2px] h-7 min-w-0 overflow-hidden",
            style: "{TRACK}",
            div { class: "flex-1 h-full min-w-3", style: "{DOTS}" }
            for (i, lvl) in bars.iter().enumerate() {
                {
                    let height = 3.0 + lvl * 23.0;
                    rsx! {
                        div {
                            key: "bar-{i}",
                            class: "w-[3px] shrink-0 rounded-full bg-white/90",
                            style: "height: {height:.1}px;",
                        }
                    }
                }
            }
        }
    }
}
