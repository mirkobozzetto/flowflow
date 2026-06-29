use crate::ui::AppState;
use dioxus::prelude::*;
use futures_timer::Delay;
use std::time::Duration;

const FADE: &str = "-webkit-mask-image: linear-gradient(90deg, transparent, #000 20px, #000 calc(100% - 20px), transparent); mask-image: linear-gradient(90deg, transparent, #000 20px, #000 calc(100% - 20px), transparent);";

#[component]
pub fn Waveform(
    #[props(default = 80)] num_bars: usize,
    #[props(default = false)] frozen: bool,
) -> Element {
    let app: AppState = use_context();
    let mut display = use_signal(|| vec![0.0f32; num_bars]);

    use_future(move || async move {
        loop {
            let target = (app.audio_levels)();
            {
                let mut cur = display.write();
                if cur.len() != num_bars {
                    cur.resize(num_bars, 0.0);
                }
                for i in 0..num_bars {
                    let t = target.get(i).copied().unwrap_or(0.0);
                    cur[i] += (t - cur[i]) * 0.3;
                }
            }
            Delay::new(Duration::from_millis(16)).await;
        }
    });

    let levels = display();

    rsx! {
        div {
            class: "flex-1 flex items-center justify-center gap-px h-7 min-w-0 overflow-hidden",
            style: "{FADE}",
            for (i, &lvl) in levels.iter().enumerate() {
                {
                    let s = if frozen { 0.16 } else { 0.16 + lvl.clamp(0.0, 1.0) * 0.84 };
                    let key = format!("bar-{i}");
                    rsx! {
                        div {
                            key: "{key}",
                            class: "flex-1 max-w-[3px] min-w-0 rounded-full",
                            style: "height: 25px; transform: scaleY({s:.3}); transform-origin: center; will-change: transform; background: rgba(255,255,255,0.92);",
                        }
                    }
                }
            }
        }
    }
}
