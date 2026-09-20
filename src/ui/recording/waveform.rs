use dioxus::prelude::*;

// Timeline of the voice capsule: a dotted track fills the left, bars grow
// from the right, the left edge is masked so old bars fade out. The bars are
// owned by voice_timeline.js (60 fps, sub-pixel scroll); Rust only pushes one
// (rms, peak) slice per tick through the `Eval` returned by `mount_timeline`.
const TRACK: &str = "-webkit-mask-image: linear-gradient(90deg, transparent, #000 12px, #000 100%); mask-image: linear-gradient(90deg, transparent, #000 12px, #000 100%);";
const DOTS: &str = "background: radial-gradient(circle, rgba(255,255,255,0.45) 1.5px, transparent 1.6px) 0 50% / 7px 3px repeat-x;";
const TIMELINE_JS: &str = include_str!("voice_timeline.js");

pub const TICK_MS: u64 = 30;

pub fn mount_timeline() -> dioxus::document::Eval {
    dioxus::document::eval(
        &TIMELINE_JS.replace("__TICK__", &TICK_MS.to_string()),
    )
}

#[component]
pub fn Waveform() -> Element {
    rsx! {
        div {
            class: "voice-timeline flex-1 flex items-center justify-end h-7 min-w-0 overflow-hidden",
            style: "{TRACK}",
            div { class: "flex-1 h-full min-w-3", style: "{DOTS}" }
            div { class: "voice-bars flex items-center gap-[2px] shrink-0 h-full will-change-transform" }
        }
    }
}
