use crate::services::audio::{AudioRecorder, RecordingState};
use crate::services::i18n::t;
use crate::ui::icons::*;
use crate::ui::recording::{start_recording, RecordingControls};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

#[component]
pub fn ChatInputBar(
    input: Signal<String>,
    disabled: bool,
    on_send: EventHandler<String>,
) -> Element {
    let app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let lang = (app.current_lang)();
    let placeholder = t(&lang, "chat-input-placeholder");
    let dummy_pending_audio: Signal<Option<(String, f64)>> =
        use_signal(|| None);

    let recording_state = (app.recording_state)();
    let is_idle = recording_state == RecordingState::Idle
        || matches!(recording_state, RecordingState::Error(_))
        || matches!(recording_state, RecordingState::Transcribed(_));

    rsx! {
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-2 bg-warm-white border-t border-stone-200 z-30 keyboard-aware lg:left-72",
            div { class: "lg:max-w-3xl lg:mx-auto",
            if is_idle {
                div { class: "flex items-end gap-2",
                    button {
                        class: "w-10 h-10 shrink-0 rounded-full bg-warm-white border border-ios-orange/15 flex items-center justify-center text-ios-orange-dark transition-colors duration-150",
                        disabled: disabled,
                        onclick: move |_| start_recording(recorder, app),
                        IconMic { size: 18 }
                    }
                    textarea {
                        class: "chat-textarea flex-1 bg-stone-100 rounded-2xl px-4 py-2.5 text-sm outline-none text-stone-900 placeholder-stone-400 resize-none overflow-y-auto",
                        style: "max-height: 120px; min-height: 40px;",
                        rows: "1",
                        placeholder: "{placeholder}",
                        value: "{input}",
                        oninput: move |evt| {
                            input.set(evt.value());
                            dioxus::document::eval(
                                r#"
                                var ta = document.querySelector('.chat-textarea');
                                if (ta) { ta.style.height = 'auto'; ta.style.height = ta.scrollHeight + 'px'; }
                                "#,
                            );
                        },
                    }
                    button {
                        class: if disabled || input().trim().is_empty() {
                            "w-10 h-10 shrink-0 rounded-full bg-stone-200 flex items-center justify-center text-stone-400 transition-colors duration-150"
                        } else {
                            "w-10 h-10 shrink-0 rounded-full bg-ios-orange flex items-center justify-center text-white transition-colors duration-150"
                        },
                        disabled: disabled || input().trim().is_empty(),
                        onclick: move |_| {
                            let q = input().trim().to_string();
                            if !q.is_empty() && !disabled {
                                input.set(String::new());
                                dioxus::document::eval(
                                    r#"
                                    var ta = document.querySelector('.chat-textarea');
                                    if (ta) { ta.style.height = 'auto'; }
                                    "#,
                                );
                                on_send.call(q);
                            }
                        },
                        IconPaperPlaneRight { size: 16 }
                    }
                }
                if let RecordingState::Error(ref e) = recording_state {
                    p { class: "text-xs text-ios-red text-center mt-1", "{e}" }
                }
            } else {
                RecordingControls { pending_audio: dummy_pending_audio, transcribe_only: true }
            }
            }
        }
    }
}
