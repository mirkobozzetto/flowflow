use crate::application::i18n::t;
use crate::ui::icons::IconDotsThree;
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn AudioPlayer(
    audio_path: String,
    duration_secs: Option<f64>,
    /// Whether this clip is the one currently playing. Playback is global - one
    /// `AVAudioPlayer` at a time - so the state lives in the parent, which is
    /// also what lets the transcript follow along.
    playing: bool,
    on_play: EventHandler<()>,
    on_stop: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let confirm_label = t(&lang, "audio-confirm-delete");
    let mut confirm_delete = use_signal(|| false);

    let dur = duration_secs.unwrap_or(0.0);
    let mins = (dur as u32) / 60;
    let secs = (dur as u32) % 60;
    let file_exists = std::path::Path::new(&audio_path).exists();
    let remote_label = t(&lang, "audio-remote-only");

    rsx! {
        div { class: "flex items-center gap-3 bg-stone-50 border border-stone-200 rounded-xl px-3 py-2 mb-2",
            button {
                class: if file_exists {
                    "w-9 h-9 rounded-full bg-ios-orange/80 flex items-center justify-center flex-shrink-0 text-white"
                } else {
                    "w-9 h-9 rounded-full bg-stone-200 flex items-center justify-center flex-shrink-0 text-stone-400"
                },
                disabled: !file_exists,
                onclick: move |_| {
                    if playing {
                        on_stop.call(());
                    } else {
                        on_play.call(());
                    }
                },
                if playing {
                    svg {
                        width: "14", height: "14", view_box: "0 0 14 14", fill: "currentColor",
                        rect { x: "1", y: "1", width: "4", height: "12", rx: "1" }
                        rect { x: "9", y: "1", width: "4", height: "12", rx: "1" }
                    }
                } else {
                    svg {
                        width: "14", height: "14", view_box: "0 0 14 14", fill: "currentColor",
                        polygon { points: "2,0 14,7 2,14" }
                    }
                }
            }
            if file_exists {
                span { class: "text-xs text-stone-500 tabular-nums flex-1", "{mins}:{secs:02}" }
            } else {
                span { class: "text-xs text-stone-400 flex-1", "{remote_label} · {mins}:{secs:02}" }
            }
            if confirm_delete() {
                button {
                    class: "text-xs text-ios-red font-medium",
                    onclick: move |_| {
                        on_stop.call(());
                        confirm_delete.set(false);
                        on_delete.call(());
                    },
                    "{confirm_label}"
                }
            } else {
                button {
                    class: "text-stone-400",
                    onclick: move |_| {
                        confirm_delete.set(true);
                        spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                            confirm_delete.set(false);
                        });
                    },
                    IconDotsThree { size: 18 }
                }
            }
        }
    }
}
