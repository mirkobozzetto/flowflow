use crate::application::i18n::t;
use crate::domain::{generate_auto_title, ChatScope, NewTextNote, Note};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{IconChatAi, IconPlus};
use crate::ui::thread::header_menu::ThreadHeaderMenu;
use crate::ui::{AppState, SidebarTab, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn ThreadDetail() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let thread_id = match (app.view)() {
        View::ThreadDetail { thread_id } => thread_id,
        _ => return rsx! {},
    };

    use_drop({
        let tid = thread_id.clone();
        move || {
            let database = db();
            if let Ok(members) = database.list_thread_notes(&tid) {
                if members.len() < 2 {
                    let _ = database.delete_thread(&tid);
                }
            }
        }
    });

    let tid_notes = thread_id.clone();
    let notes = use_memo(move || {
        let _ = (app.notes_version)();
        let _ = (app.sync_data_version)();
        db().list_thread_notes(&tid_notes).unwrap_or_default()
    });

    let theme = {
        let _ = (app.folders_version)();
        db().get_thread(&thread_id)
            .ok()
            .flatten()
            .and_then(|t| t.folder_id)
            .and_then(|fid| db().get_folder(&fid).ok().flatten())
            .map(|f| f.name)
    };

    let tid_add = thread_id.clone();
    let lang_add = lang.clone();
    let add_note = move |_: Event<MouseData>| {
        app.show_thread_menu.set(false);
        if let Ok(note) = db().create_text_note_in_thread(
            &NewTextNote {
                title: Some(generate_auto_title(&lang_add)),
                content: String::new(),
                tags: vec![],
            },
            &tid_add,
        ) {
            let _ = db().touch_thread(&tid_add);
            app.notes_version.set((app.notes_version)() + 1);
            app.previous_view.set(Some(View::ThreadDetail {
                thread_id: tid_add.clone(),
            }));
            app.view.set(View::NoteDetail { note_id: note.id });
        }
    };

    let tid_chat = thread_id.clone();
    let open_chat = move |_: Event<MouseData>| {
        app.show_thread_menu.set(false);
        app.chat_scope
            .set(Some(ChatScope::Thread(tid_chat.clone())));
        app.sidebar_tab.set(SidebarTab::Chats);
        app.previous_view.set(Some(View::ThreadDetail {
            thread_id: tid_chat.clone(),
        }));
        app.view.set(View::Chat {
            conversation_id: None,
        });
    };

    rsx! {
        if (app.show_thread_menu)() {
            ThreadHeaderMenu { thread_id: thread_id.clone() }
        }
        div { class: "flex-1 overflow-y-auto px-4 pt-4 safe-pb-32 lg:px-[max(1rem,calc((100%-48rem)/2))]",
            if let Some(ref tname) = theme {
                div { class: "mb-4",
                    span { class: "px-2.5 py-1 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-xs font-medium",
                        "{tname}"
                    }
                }
            }
            if notes().is_empty() {
                div { class: "flex flex-col items-center justify-center gap-2 h-[40vh]",
                    p { class: "text-stone-400 text-sm", {t(&lang, "thread-empty")} }
                }
            } else {
                div {
                    for note in notes() {
                        ThreadNode { key: "{note.id}", thread_id: thread_id.clone(), note: note }
                    }
                }
            }
        }
        div { class: "fixed bottom-0 left-0 right-0 px-4 py-2 bg-warm-white border-t border-stone-200 z-30 keyboard-aware lg:left-72",
            div { class: "lg:max-w-3xl lg:mx-auto",
                div { class: "flex items-center gap-2",
                    button {
                        class: "flex-1 flex items-center justify-center gap-2.5 h-12 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-sm font-medium",
                        onclick: add_note,
                        IconPlus { size: 22 }
                        span { {t(&lang, "thread-add")} }
                    }
                    button {
                        class: "shrink-0 w-12 h-12 flex items-center justify-center rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark active:opacity-70",
                        "aria-label": t(&lang, "note-chat-entry"),
                        onclick: open_chat,
                        IconChatAi { size: 24 }
                    }
                }
            }
        }
    }
}

#[component]
fn ThreadNode(thread_id: String, note: Note) -> Element {
    let mut app: AppState = use_context();
    let lang = (app.current_lang)();
    let nid = note.id.clone();
    let date = note.created_at.get(..10).unwrap_or("").to_string();
    let time = note.created_at.get(11..16).unwrap_or("").to_string();
    let title = note
        .title
        .clone()
        .unwrap_or_else(|| t(&lang, "note-card-untitled"));

    rsx! {
        div { class: "flex gap-3 pb-4",
            div { class: "flex flex-col items-center pt-1.5",
                div { class: "w-2.5 h-2.5 rounded-full bg-ios-orange shrink-0" }
                div { class: "flex-1 w-px bg-stone-200 mt-1" }
            }
            div {
                class: "flex-1 bg-warm-white border border-stone-200 rounded-xl p-3 cursor-pointer hover:border-stone-300 transition-colors duration-150",
                onclick: move |_| {
                    app.show_folder_picker.set(false);
                    app.previous_view.set(Some(View::ThreadDetail {
                        thread_id: thread_id.clone(),
                    }));
                    app.view.set(View::NoteDetail { note_id: nid.clone() });
                },
                div { class: "flex justify-between items-baseline mb-1 gap-2",
                    h4 { class: "font-medium text-sm text-stone-900", "{title}" }
                    span { class: "text-xs text-stone-400 shrink-0", "{date} {time}" }
                }
                if !note.content.is_empty() {
                    p { class: "text-stone-700 text-sm whitespace-pre-wrap break-words line-clamp-6",
                        "{note.content}"
                    }
                }
            }
        }
    }
}
