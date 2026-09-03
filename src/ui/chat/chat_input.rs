use crate::application::i18n::t;
use crate::infrastructure::audio::{AudioRecorder, RecordingState};
use crate::ui::chat::mention_menu::{MentionMenu, MentionedNote};
use crate::ui::chat::tools_menu::ToolsMenu;
use crate::ui::icons::*;
use crate::ui::recording::{start_recording, RecordingControls};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

// A mention is the trailing "@frag" at the very end of the input (start of input
// or after whitespace, no space yet). Returns the fragment after "@" to filter on.
fn mention_fragment(s: &str) -> Option<String> {
    let at = s.rfind('@')?;
    if at > 0 {
        let prev = s[..at].chars().next_back()?;
        if !prev.is_whitespace() {
            return None;
        }
    }
    let frag = &s[at + 1..];
    if frag.chars().any(char::is_whitespace) {
        return None;
    }
    Some(frag.to_string())
}

#[component]
pub fn ChatInputBar(
    input: Signal<String>,
    mentions: Signal<Vec<MentionedNote>>,
    disabled: bool,
    on_send: EventHandler<String>,
) -> Element {
    let mut app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let lang = (app.current_lang)();
    let placeholder = t(&lang, "chat-input-placeholder");
    let dummy_pending_audio: Signal<Option<(String, f64)>> =
        use_signal(|| None);
    let mut mention_query = use_signal(String::new);
    let mut plus_hover = use_signal(|| false);

    let recording_state = (app.recording_state)();
    let is_idle = recording_state == RecordingState::Idle
        || matches!(recording_state, RecordingState::Error(_))
        || matches!(recording_state, RecordingState::Transcribed { .. });

    let show_tools = (app.show_tools_menu)();
    let show_mention = (app.show_mention_menu)();
    let is_desktop = !cfg!(target_os = "ios");

    // The + palette drops a ready-to-run command here ("lance <alias>"): PREFILL the input
    // so the user can complete it (add the goal, correct the target) and send deliberately -
    // a tap must never fire an agent run on its own.
    use_effect(move || {
        if let Some(cmd) = (app.pending_chat_input)() {
            let mut app = app;
            app.pending_chat_input.set(None);
            if !cmd.trim().is_empty() {
                input.set(format!("{cmd} "));
            }
        }
    });

    let mut do_send = move || {
        let q = input().trim().to_string();
        if !q.is_empty() && !disabled {
            input.set(String::new());
            app.show_tools_menu.set(false);
            app.show_mention_menu.set(false);
            dioxus::document::eval(
                r#"
                var ta = document.querySelector('.chat-textarea');
                if (ta) { ta.style.height = 'auto'; }
                "#,
            );
            on_send.call(q);
        }
    };

    rsx! {
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-2 bg-warm-white border-t border-stone-200 z-30 keyboard-aware lg:left-72",
            div { class: "lg:max-w-3xl lg:mx-auto",
            if is_idle {
                div { class: "relative flex items-end gap-2",
                    if show_mention {
                        MentionMenu {
                            input: input,
                            mentions: mentions,
                            query: mention_query(),
                        }
                    }
                    div { class: "relative shrink-0",
                        button {
                            class: "pressable w-11 h-11 lg:w-12 lg:h-12 rounded-full bg-warm-white border border-stone-200 flex items-center justify-center text-stone-600 hover:bg-stone-100",
                            disabled: disabled,
                            onmouseenter: move |_| plus_hover.set(true),
                            onmouseleave: move |_| plus_hover.set(false),
                            onclick: move |_| {
                                app.show_mention_menu.set(false);
                                app.show_tools_menu.set(!(app.show_tools_menu)());
                            },
                            onkeydown: move |evt| {
                                if evt.key() == Key::Escape {
                                    app.show_tools_menu.set(false);
                                }
                            },
                            IconPlus { size: 20 }
                        }
                        if show_tools {
                            ToolsMenu {}
                        }
                        if is_desktop && plus_hover() && !show_tools {
                            div { class: "absolute bottom-full left-0 mb-2 z-40 pointer-events-none flex items-center gap-2 whitespace-nowrap bg-stone-800 text-white text-xs font-medium px-2.5 py-1.5 rounded-lg shadow tooltip-fade",
                                span { {t(&lang, "chat-tools-tooltip")} }
                                span { class: "px-1.5 py-0.5 rounded bg-stone-700 border border-stone-600 font-mono text-[11px]", "@" }
                            }
                        }
                    }
                    button {
                        class: "pressable w-11 h-11 lg:w-12 lg:h-12 shrink-0 rounded-full bg-warm-white border border-ios-orange/15 flex items-center justify-center text-ios-orange-dark hover:bg-ios-orange-50",
                        disabled: disabled,
                        onclick: move |_| start_recording(recorder, app),
                        IconMic { size: 18 }
                    }
                    textarea {
                        class: "chat-textarea flex-1 min-h-[44px] lg:min-h-[48px] bg-stone-100 border border-transparent rounded-2xl px-4 py-3 lg:py-3.5 text-sm outline-none text-stone-900 placeholder:text-stone-400 resize-none overflow-y-auto focus:bg-warm-white focus:ring-[3px] focus:ring-ios-orange-50 focus:border-ios-orange-dark transition-[background-color,border-color,box-shadow] duration-150",
                        style: "max-height: 120px;",
                        rows: "1",
                        placeholder: "{placeholder}",
                        value: "{input}",
                        oninput: move |evt| {
                            let val = evt.value();
                            input.set(val.clone());
                            mentions.write().retain(|m| val.contains(&format!("@{}", m.title)));
                            match mention_fragment(&val) {
                                Some(frag) => {
                                    mention_query.set(frag);
                                    app.show_tools_menu.set(false);
                                    app.show_mention_menu.set(true);
                                }
                                None => app.show_mention_menu.set(false),
                            }
                            dioxus::document::eval(
                                r#"
                                var ta = document.querySelector('.chat-textarea');
                                if (ta) { ta.style.height = 'auto'; ta.style.height = ta.scrollHeight + 'px'; }
                                "#,
                            );
                        },
                        onkeydown: move |evt| {
                            if evt.key() == Key::Escape {
                                app.show_tools_menu.set(false);
                                app.show_mention_menu.set(false);
                            }
                            if cfg!(target_os = "ios") {
                                return;
                            }
                            if evt.key() == Key::Enter
                                && !evt.modifiers().shift()
                            {
                                evt.prevent_default();
                                do_send();
                            }
                        },
                    }
                    button {
                        class: if disabled || input().trim().is_empty() {
                            "pressable w-11 h-11 lg:w-12 lg:h-12 shrink-0 rounded-full bg-stone-200 flex items-center justify-center text-stone-400"
                        } else {
                            "pressable w-11 h-11 lg:w-12 lg:h-12 shrink-0 rounded-full bg-ios-orange flex items-center justify-center text-white hover:bg-ios-orange-dark active:scale-[0.98]"
                        },
                        disabled: disabled || input().trim().is_empty(),
                        onclick: move |_| do_send(),
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
