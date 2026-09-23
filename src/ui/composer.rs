use crate::application::i18n::t;
use crate::infrastructure::audio::{AudioRecorder, RecordingState};
use crate::infrastructure::platform::{haptic, haptic_prepare};
use crate::ui::chat::mention_menu::{MentionMenu, MentionedNote};
use crate::ui::chat::tools_menu::ToolsMenu;
use crate::ui::icons::*;
use crate::ui::recording::{start_recording, VoiceCapsule};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq)]
pub enum ComposerRole {
    /// The note bar: text is appended to the open note's body.
    AppendToNote,
    /// The chat bar: text is sent as a message.
    SendMessage,
}

// A mention is the trailing "@frag" at the very end of the input (start of input
// or after whitespace, no space yet). Returns the fragment after "@" to filter on.
pub fn mention_fragment(s: &str) -> Option<String> {
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

// Fit the field, then size the CAPSULE: it is the only animated box (height and
// radius transition in CSS), the buttons stay anchored to its bottom and the
// field slides to full width on the second line. The caret never moves: the
// box comes to it. Validated on ~/ff-ux-mockup/composer-multiline.html.
const AUTOSIZE: &str = r#"
    var ta = document.querySelector('.composer-field');
    var cap = ta && ta.closest('.composer-capsule');
    if (ta && cap) {
        ta.style.height = 'auto';
        var h = Math.min(ta.scrollHeight, 160);
        ta.style.height = h + 'px';
        var multi = h > 44;
        cap.setAttribute('data-multi', multi);
        cap.style.height = (multi ? 6 + h + 50 : 56) + 'px';
    }
"#;

// One capsule for the note and the chat: "+", the field, one orange button (mic
// when empty, arrow when there is text). `children` render outside the capsule,
// to its right (the note's thread and chat entry buttons).
#[component]
pub fn Composer(
    role: ComposerRole,
    input: Signal<String>,
    #[props(default = false)] disabled: bool,
    on_commit: EventHandler<String>,
    pending_audio: Signal<Option<(String, f64)>>,
    /// Chat only: the notes mentioned with "@" in the field.
    #[props(default)]
    mentions: Option<Signal<Vec<MentionedNote>>>,
    children: Element,
) -> Element {
    let mut app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let lang = (app.current_lang)();
    let chat = role == ComposerRole::SendMessage;
    let mut mention_query = use_signal(String::new);
    let mut focused = use_signal(|| false);
    let mut commit_on_transcribed = use_signal(|| false);
    // Exit choreography: the voice capsule stays mounted 320 ms after the
    // take ends so CSS can play it out; the field flashes when text lands.
    let mut voice_leaving = use_signal(|| false);
    // Enter transition: false on the frame the layer mounts, true one tick
    // later. Dioxus owns every attribute of that node, so the flag must be a
    // Rust signal, never something JS sets on the element.
    let mut voice_in = use_signal(|| false);
    let landed = use_signal(|| false);
    let sent = use_signal(|| false);

    let recording_state = (app.recording_state)();
    let is_idle = recording_state == RecordingState::Idle
        || matches!(recording_state, RecordingState::Error(_))
        || matches!(recording_state, RecordingState::Transcribed { .. });
    let mut was_live = use_signal(|| false);
    use_effect(move || {
        // Read the signal here: `is_idle` is a plain local of this render,
        // so an effect built on it alone never re-runs.
        let state = (app.recording_state)();
        let live = !(state == RecordingState::Idle
            || matches!(state, RecordingState::Error(_))
            || matches!(state, RecordingState::Transcribed { .. }));
        // peek: an effect must never subscribe to the signal it writes.
        let before = *was_live.peek();
        if before == live {
            return;
        }
        was_live.set(live);
        if live {
            voice_in.set(false);
            spawn(async move {
                futures_timer::Delay::new(std::time::Duration::from_millis(16))
                    .await;
                voice_in.set(true);
            });
        }
        if before && !live {
            voice_leaving.set(true);
            spawn(async move {
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    320,
                ))
                .await;
                voice_leaving.set(false);
            });
        }
    });
    let flash = move |signal: Signal<bool>| {
        let mut signal = signal;
        signal.set(true);
        spawn(async move {
            futures_timer::Delay::new(std::time::Duration::from_millis(320))
                .await;
            signal.set(false);
        });
    };
    let menu_open = if chat {
        (app.show_tools_menu)()
    } else {
        (app.show_note_tools_menu)()
    };
    let show_mention = chat && (app.show_mention_menu)();
    let empty = input().trim().is_empty();

    let mut commit = move || {
        let text = input().trim().to_string();
        if text.is_empty() || disabled {
            return;
        }
        input.set(String::new());
        flash(sent);
        app.show_tools_menu.set(false);
        app.show_note_tools_menu.set(false);
        app.show_mention_menu.set(false);
        dioxus::document::eval(AUTOSIZE);
        on_commit.call(text);
    };

    // Dictation lands here: the square puts the text in the field for review,
    // the arrow commits it straight away.
    use_effect(move || {
        if let RecordingState::Transcribed { transcript } =
            (app.recording_state)()
        {
            let text = transcript.text();
            let current = input.peek().clone();
            input.set(if current.is_empty() {
                text
            } else {
                format!("{current} {text}")
            });
            app.recording_state.set(RecordingState::Idle);
            haptic("light");
            if *commit_on_transcribed.peek() {
                commit_on_transcribed.set(false);
                commit();
            } else {
                flash(landed);
                dioxus::document::eval(&format!(
                    "requestAnimationFrame(() => {{ {AUTOSIZE} var f = document.querySelector('.composer-field'); if (f) f.focus(); }});"
                ));
            }
        }
    });

    // The + palette drops a ready-to-run command here ("lance <alias>"): PREFILL the
    // input so the user can complete it and send deliberately.
    use_effect(move || {
        if !chat {
            return;
        }
        if let Some(cmd) = (app.pending_chat_input)() {
            app.pending_chat_input.set(None);
            if !cmd.trim().is_empty() {
                input.set(format!("{cmd} "));
            }
        }
    });

    let placeholder = t(
        &lang,
        if chat {
            "chat-input-placeholder"
        } else {
            "composer-note-placeholder"
        },
    );
    let capsule = if focused() {
        "composer-capsule relative h-14 rounded-[28px] bg-warm-white border border-ios-orange-dark ring-[3px] ring-ios-orange-50"
    } else {
        "composer-capsule relative h-14 rounded-[28px] bg-stone-100 border border-transparent"
    };

    rsx! {
        div { class: "fixed bottom-0 left-0 right-0 px-3 py-2 bg-warm-white border-t border-stone-200 z-30 keyboard-aware lg:left-72",
            div { class: "lg:max-w-3xl lg:mx-auto",
                div { class: "relative flex items-end gap-2",
                    if let (true, Some(mentions)) = (show_mention, mentions) {
                        MentionMenu { input, mentions, query: mention_query() }
                    }
                    div { class: "composer-stack relative flex-1 min-w-0",
                        div { class: capsule,
                            "data-hidden": !is_idle,
                            "data-landed": landed(),
                            div { class: "absolute left-[5px] bottom-[5px]",
                                button {
                                    class: "composer-plus pressable w-11 h-11 rounded-full flex items-center justify-center text-stone-600 hover:bg-stone-200/70",
                                    "data-open": menu_open,
                                    "aria-label": t(&lang, "chat-tools-tooltip"),
                                    "aria-expanded": menu_open,
                                    disabled: disabled,
                                    onclick: move |_| {
                                        app.show_mention_menu.set(false);
                                        if chat {
                                            app.show_tools_menu.set(!(app.show_tools_menu)());
                                        } else {
                                            app.show_note_tools_menu.set(!(app.show_note_tools_menu)());
                                        }
                                    },
                                    IconPlus { size: 22 }
                                }
                                if menu_open {
                                    ToolsMenu { note: !chat }
                                }
                            }
                            textarea {
                                class: "composer-field absolute top-1.5 left-1.5 right-1.5 max-h-40 bg-transparent border-0 px-1.5 py-[11px] text-[15px] leading-[1.4] outline-none text-stone-900 placeholder:text-stone-400 resize-none overflow-y-auto",
                                rows: "1",
                                placeholder: "{placeholder}",
                                value: "{input}",
                                disabled: disabled,
                                onfocus: move |_| focused.set(true),
                                onblur: move |_| focused.set(false),
                                oninput: move |evt| {
                                    let val = evt.value();
                                    input.set(val.clone());
                                    if let Some(mut mentions) = mentions {
                                        mentions.write().retain(|m| val.contains(&format!("@{}", m.title)));
                                        match mention_fragment(&val) {
                                            Some(frag) => {
                                                mention_query.set(frag);
                                                app.show_tools_menu.set(false);
                                                app.show_mention_menu.set(true);
                                            }
                                            None => app.show_mention_menu.set(false),
                                        }
                                    }
                                    dioxus::document::eval(AUTOSIZE);
                                },
                                onkeydown: move |evt| {
                                    if evt.key() == Key::Escape {
                                        app.show_tools_menu.set(false);
                                        app.show_note_tools_menu.set(false);
                                        app.show_mention_menu.set(false);
                                    }
                                    if cfg!(target_os = "ios") {
                                        return;
                                    }
                                    if evt.key() == Key::Enter && !evt.modifiers().shift() {
                                        evt.prevent_default();
                                        commit();
                                    }
                                },
                            }
                            button {
                                class: "composer-primary pressable press-grow absolute right-1 bottom-1 w-12 h-12 rounded-full bg-ios-orange text-white flex items-center justify-center overflow-hidden disabled:opacity-50",
                                "data-has-text": !empty,
                                "data-sent": sent(),
                                "aria-label": t(&lang, if empty { "recording-dictate" } else if chat { "chat-send" } else { "composer-append" }),
                                disabled: disabled,
                                onpointerdown: move |_| if empty { haptic_prepare("medium") } else { haptic_prepare("soft") },
                                onclick: move |_| {
                                    if empty {
                                        haptic("medium");
                                        start_recording(recorder, app);
                                    } else {
                                        haptic("soft");
                                        commit();
                                    }
                                },
                                span { class: "composer-icon composer-icon-mic absolute inset-0 flex items-center justify-center", IconMic { size: 20 } }
                                span { class: "composer-icon composer-icon-send absolute inset-0 flex items-center justify-center", IconArrowUp { size: 20 } }
                            }
                        }
                        if !is_idle || voice_leaving() {
                            // Enter/exit are CSS transitions driven by Rust flags:
                            // data-in flips one tick after mount, data-leaving on exit.
                            div { class: "voice-layer absolute inset-0",
                                "data-in": voice_in() && !is_idle,
                                "data-leaving": is_idle,
                                VoiceCapsule { pending_audio, transcribe_only: chat, commit_on_transcribed }
                            }
                        }
                    }
                    {children}
                }
                if let RecordingState::Error(ref e) = recording_state {
                    p { class: "text-xs text-ios-red text-center mt-1", "{e}" }
                }
            }
        }
    }
}
