use crate::ui::icons::{IconPause, IconPlay, IconTrash};
use dioxus::prelude::*;

#[component]
pub fn AudioPlayer(
    audio_path: String,
    duration_secs: Option<f64>,
    on_delete: EventHandler<()>,
) -> Element {
    let mut playing = use_signal(|| false);

    let dur = duration_secs.unwrap_or(0.0);
    let mins = (dur as u32) / 60;
    let secs = (dur as u32) % 60;

    rsx! {
        div { class: "flex items-center gap-3 bg-stone-50 border border-stone-200 rounded-xl px-3 py-2 mb-2",
            button {
                class: "w-8 h-8 rounded-full bg-ios-green flex items-center justify-center flex-shrink-0 text-white",
                onclick: move |_| {
                    if playing() {
                        #[cfg(target_os = "ios")]
                        crate::platform::ios::stop_audio();
                        playing.set(false);
                    } else {
                        #[cfg(target_os = "ios")]
                        crate::platform::ios::play_audio(&audio_path);
                        playing.set(true);
                    }
                },
                if playing() {
                    IconPause { size: 14 }
                } else {
                    IconPlay { size: 14 }
                }
            }
            span { class: "text-xs text-stone-500 tabular-nums flex-1", "{mins}:{secs:02}" }
            button {
                class: "text-ios-red hover:opacity-70",
                onclick: move |_| {
                    #[cfg(target_os = "ios")]
                    crate::platform::ios::stop_audio();
                    playing.set(false);
                    on_delete.call(());
                },
                IconTrash { size: 16 }
            }
        }
    }
}
