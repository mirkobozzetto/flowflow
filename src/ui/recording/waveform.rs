use crate::ui::AppState;
use dioxus::prelude::*;
use futures_timer::Delay;
use std::time::Duration;

// Timeline of the voice capsule: a dotted track fills the left, one bar per
// 90 ms grows from the right, the left edge is masked so old bars fade out.
const TRACK: &str = "-webkit-mask-image: linear-gradient(90deg, transparent, #000 12px, #000 100%); mask-image: linear-gradient(90deg, transparent, #000 12px, #000 100%);";
const DOTS: &str = "background: radial-gradient(circle, rgba(255,255,255,0.45) 1.5px, transparent 1.6px) 0 50% / 7px 3px repeat-x;";

#[component]
pub fn Waveform(#[props(default = 40)] num_bars: usize) -> Element {
    let app: AppState = use_context();
    let mut history: Signal<Vec<(u64, f32)>> = use_signal(Vec::new);
    let mut seq = use_signal(|| 0u64);

    use_future(move || async move {
        loop {
            let levels = (app.audio_levels)();
            let level = if levels.is_empty() {
                0.0
            } else {
                levels.iter().sum::<f32>() / levels.len() as f32
            };
            {
                let id = seq() + 1;
                seq.set(id);
                let mut bars = history.write();
                bars.push((id, level.clamp(0.0, 1.0)));
                if bars.len() > num_bars {
                    bars.remove(0);
                }
            }
            Delay::new(Duration::from_millis(90)).await;
        }
    });

    let bars = history();

    rsx! {
        div {
            class: "flex-1 flex items-center justify-end gap-[2px] h-7 min-w-0 overflow-hidden",
            style: "{TRACK}",
            div { class: "flex-1 h-full min-w-3", style: "{DOTS}" }
            for (id, lvl) in bars {
                {
                    let height = 4.0 + lvl * 20.0;
                    rsx! {
                        div {
                            key: "bar-{id}",
                            class: "w-[3px] shrink-0 rounded-full bg-white/90",
                            style: "height: {height:.1}px; animation: fadeIn 240ms var(--ease-soft);",
                        }
                    }
                }
            }
        }
    }
}
