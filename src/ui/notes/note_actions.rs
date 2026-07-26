use crate::application::constants::NOTE_ACTION_PROMPT;
use crate::application::i18n::t;
use crate::domain::NewTextNote;
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::action_card::ActionResultCard;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

fn action_key(note_id: &str) -> String {
    format!("note_action:{note_id}")
}

// Run-this-note-as-an-action. The note text IS the instruction: it goes through the agent with the
// connected tools (NOTE_ACTION_PROMPT keeps the reply to a one-line confirmation + resource link).
// The result is cached per note so reopening the note shows what it produced instead of re-offering
// a fresh run. Dark unless a backend connector is configured.
#[component]
pub fn NoteActions(
    mut local_note_id: Signal<String>,
    title: Signal<String>,
    content: Signal<String>,
    tags: Signal<Vec<String>>,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let backend_on = use_memo(move || BackendClient::from_db(&db()).is_some());

    let mut running = use_signal(|| false);
    let mut result: Signal<Option<String>> = use_signal(|| None);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    // surface any previously cached result for this note (persistence across reopen)
    use_effect(move || {
        let nid = local_note_id();
        let stored = if nid.is_empty() {
            None
        } else {
            db().get_setting(&action_key(&nid))
        };
        result.set(stored);
    });

    let has_result = result().is_some();
    let run_label = if has_result {
        t(&lang, "note-rerun-action")
    } else {
        t(&lang, "note-run-action")
    };
    let running_label = t(&lang, "note-running-action");

    rsx! {
        if backend_on() && crate::application::intent::is_actionable(&content()) {
            div { class: "mt-3",
                button {
                    class: "w-full min-h-[44px] flex items-center justify-center gap-2 rounded-xl bg-ios-orange/10 text-ios-orange-dark text-sm font-medium active:bg-ios-orange/25 disabled:opacity-50 transition-colors duration-150",
                    disabled: running(),
                    onclick: move |_| {
                        if running() {
                            return;
                        }
                        running.set(true);
                        error.set(None);
                        let c = content();
                        let t_in = title();
                        let tg = tags();
                        let fid = (app.detail_folder_id)();
                        spawn(async move {
                            let database = db();
                            let nid = {
                                let cur = local_note_id();
                                if cur.is_empty() {
                                    let new = NewTextNote {
                                        title: if t_in.is_empty() { None } else { Some(t_in) },
                                        content: c.clone(),
                                        tags: tg,
                                    };
                                    match database.create_text_note(&new) {
                                        Ok(created) => {
                                            if let Some(ref f) = fid {
                                                let _ = database.add_note_to_folder(&created.id, f);
                                            }
                                            local_note_id.set(created.id.clone());
                                            app.current_note_id.set(Some(created.id.clone()));
                                            app.notes_version.set((app.notes_version)() + 1);
                                            created.id
                                        }
                                        Err(e) => {
                                            error.set(Some(e));
                                            running.set(false);
                                            return;
                                        }
                                    }
                                } else {
                                    cur
                                }
                            };
                            // No event channel here = no approval card can render, so this
                            // path gets the notes-only surface (connector writes need chat).
                            let outcome = match LlmClient::from_db(&database) {
                                Ok(ai) => {
                                    let ai = Arc::new(ai);
                                    crate::application::chat_surface::prompt_chat_agent(
                                        ai,
                                        NOTE_ACTION_PROMPT,
                                        &c,
                                        None,
                                        crate::infrastructure::llm::NotesTools::Global,
                                    )
                                    .await
                                    .map_err(|e| e.to_string())
                                }
                                Err(e) => Err(e.to_string()),
                            };
                            match outcome {
                                Ok(answer) => {
                                    let _ = database.set_setting(&action_key(&nid), &answer);
                                    result.set(Some(answer));
                                }
                                Err(e) => error.set(Some(e)),
                            }
                            running.set(false);
                        });
                    },
                    if running() {
                        span { class: "inline-block w-3.5 h-3.5 border-2 border-ios-orange-dark border-t-transparent rounded-full animate-spin" }
                        span { "{running_label}" }
                    } else {
                        span { "{run_label}" }
                    }
                }
                {
                    if let Some(answer) = result() {
                        rsx! {
                            ActionResultCard { text: answer }
                        }
                    } else {
                        rsx! {}
                    }
                }
                {
                    if let Some(e) = error() {
                        rsx! {
                            div {
                                class: "mt-2 px-3 py-2 bg-ios-red/10 rounded-lg text-xs text-stone-600",
                                "{e}"
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }
            }
        }
    }
}
