use crate::ui::AppState;
use dioxus::prelude::*;

const FADE_BARS: usize = 5;

fn bar_opacity(i: usize, total: usize) -> f32 {
    let from_start = i as f32;
    let from_end = (total - 1 - i) as f32;
    let edge = from_start.min(from_end);
    (0.15 + (edge / FADE_BARS as f32) * 0.85).min(1.0)
}

#[component]
pub fn Waveform(
    #[props(default = 28)] num_bars: usize,
    #[props(default = false)] frozen: bool,
) -> Element {
    let app: AppState = use_context();
    let levels = (app.audio_levels)();

    rsx! {
        div { class: "flex-1 flex items-center justify-center gap-[2px] h-7 min-w-0",
            for (i, &lvl) in levels.iter().enumerate() {
                {
                    let opacity = bar_opacity(i, num_bars.max(1));
                    let h = if frozen { 4.0 } else { 3.0 + lvl * 22.0 };
                    let key = format!("bar-{i}");
                    rsx! {
                        div {
                            key: "{key}",
                            class: "flex-1 rounded-full transition-all duration-100 ease-out",
                            style: "height: {h:.0}px; opacity: {opacity:.2}; background: rgba(255,255,255,0.8);",
                        }
                    }
                }
            }
        }
    }
}
