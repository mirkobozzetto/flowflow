use crate::services::audio::{AudioRecorder, RecordingState};
use crate::services::i18n::t;
use crate::ui::icons::*;
use crate::ui::recording::{start_recording, RecordingControls};
use crate::ui::state::View;
use crate::ui::{AppState, SidebarTab};
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};

#[component]
pub fn RecordingBar(pending_audio: Signal<Option<(String, f64)>>) -> Element {
    let mut app: AppState = use_context();
    let recorder: Signal<Arc<Mutex<AudioRecorder>>> = use_context();
    let recording_state = (app.recording_state)();
    let is_idle = recording_state == RecordingState::Idle
        || matches!(recording_state, RecordingState::Error(_))
        || matches!(recording_state, RecordingState::Transcribed(_));
    let lang = (app.current_lang)();
    let note_id_for_chat = (app.current_note_id)().filter(|id| !id.is_empty());

    rsx! {
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-2 bg-warm-white border-t border-stone-200 z-30 keyboard-aware lg:left-72",
            div { class: "lg:max-w-3xl lg:mx-auto",
            if is_idle {
                div { class: "flex items-center gap-2",
                    button {
                        class: "flex-1 flex items-center justify-center gap-2.5 h-12 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-sm font-medium",
                        onclick: move |_| start_recording(recorder, app),
                        IconMic { size: 22 }
                        span { {t(&lang, "recording-dictate")} }
                    }
                    if let Some(nid) = note_id_for_chat {
                        crate::ui::thread::ThreadEntryButton { note_id: nid.clone() }
                        button {
                            class: "shrink-0 w-12 h-12 flex items-center justify-center rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark active:opacity-70",
                            "aria-label": t(&lang, "note-chat-entry"),
                            onclick: move |_| {
                                app.show_folder_picker.set(false);
                                app.show_note_menu.set(false);
                                app.sidebar_tab.set(SidebarTab::Chats);
                                app.chat_scope.set(
                                    (app.detail_folder_id)()
                                        .map(crate::models::ChatScope::Folder),
                                );
                                app.previous_view
                                    .set(Some(View::NoteDetail { note_id: nid.clone() }));
                                app.view.set(View::Chat { conversation_id: None });
                            },
                            IconChatAi { size: 24 }
                        }
                    }
                }
                if let RecordingState::Error(ref e) = recording_state {
                    p { class: "text-xs text-ios-red text-center mt-1", "{e}" }
                }
            } else {
                RecordingControls { pending_audio }
            }
            }
        }
    }
}
