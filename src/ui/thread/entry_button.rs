use crate::db::Database;
use crate::models::{generate_auto_title, NewThread};
use crate::services::i18n::t;
use crate::ui::icons::{IconCardsThree, IconStackPlus};
use crate::ui::kit;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ThreadEntryButton(note_id: String) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut show_picker = use_signal(|| false);

    let _ = (app.notes_version)();
    let note = db().get_note(&note_id).ok().flatten();
    let in_thread = note.as_ref().and_then(|n| n.thread_id.clone());
    let note_title = note
        .as_ref()
        .and_then(|n| n.title.clone())
        .unwrap_or_default();
    let threads = db().list_threads().unwrap_or_default();

    let open_thread = {
        let note_id = note_id.clone();
        move |tid: String| {
            app.previous_view.set(Some(View::NoteDetail {
                note_id: note_id.clone(),
            }));
            app.view.set(View::ThreadDetail { thread_id: tid });
        }
    };

    let start_thread = {
        let note_id = note_id.clone();
        let note_title = note_title.clone();
        let lang = lang.clone();
        move || {
            let title = if note_title.trim().is_empty() {
                generate_auto_title(&lang)
            } else {
                note_title.clone()
            };
            let folder_id = db()
                .folders_for_note(&note_id)
                .ok()
                .and_then(|f| f.first().map(|f| f.id.clone()));
            if let Ok(thread) =
                db().create_thread(&NewThread { title, folder_id })
            {
                let _ = db().add_note_to_thread(&note_id, &thread.id);
                app.notes_version.set((app.notes_version)() + 1);
                app.previous_view.set(Some(View::NoteDetail {
                    note_id: note_id.clone(),
                }));
                app.view.set(View::ThreadDetail {
                    thread_id: thread.id,
                });
            }
        }
    };

    let in_thread_btn = in_thread.clone();
    let threads_empty = threads.is_empty();

    rsx! {
        div { class: "relative shrink-0",
            if show_picker() {
                div {
                    class: "fixed inset-0 z-30",
                    onclick: move |_| show_picker.set(false),
                }
                div { class: "absolute bottom-full right-0 mb-2 z-40 min-w-[230px] max-h-[55vh] overflow-y-auto {kit::MENU_PANEL}",
                    button {
                        class: kit::MENU_ITEM,
                        onclick: {
                            let mut start = start_thread.clone();
                            move |_| {
                                show_picker.set(false);
                                start();
                            }
                        },
                        IconStackPlus { size: 16 }
                        {t(&lang, "thread-start")}
                    }
                    if !threads.is_empty() {
                        div { class: kit::MENU_SEP }
                        p { class: "{kit::SECTION_LABEL} px-3 pb-1", {t(&lang, "thread-add-to")} }
                        for thread in threads.iter() {
                            button {
                                key: "{thread.id}",
                                class: kit::MENU_ITEM,
                                onclick: {
                                    let note_id = note_id.clone();
                                    let tid = thread.id.clone();
                                    move |_| {
                                        let _ = db().add_note_to_thread(&note_id, &tid);
                                        app.notes_version.set((app.notes_version)() + 1);
                                        show_picker.set(false);
                                        app.previous_view.set(Some(View::NoteDetail {
                                            note_id: note_id.clone(),
                                        }));
                                        app.view.set(View::ThreadDetail { thread_id: tid.clone() });
                                    }
                                },
                                IconCardsThree { size: 16 }
                                {
                                    if thread.title.is_empty() {
                                        t(&lang, "thread-untitled")
                                    } else {
                                        thread.title.clone()
                                    }
                                }
                            }
                        }
                    }
                }
            }
            button {
                class: if in_thread_btn.is_some() {
                    "w-12 h-12 flex items-center justify-center rounded-full bg-ios-orange/15 border border-ios-orange/40 text-ios-orange-dark active:opacity-70"
                } else {
                    "w-12 h-12 flex items-center justify-center rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark active:opacity-70"
                },
                "aria-label": if in_thread_btn.is_some() { t(&lang, "thread-open") } else { t(&lang, "thread-add") },
                onclick: move |_| {
                    if let Some(tid) = in_thread_btn.clone() {
                        let mut open = open_thread.clone();
                        open(tid);
                    } else if threads_empty {
                        let mut start = start_thread.clone();
                        start();
                    } else {
                        show_picker.set(true);
                    }
                },
                IconCardsThree { size: 24 }
            }
        }
    }
}
