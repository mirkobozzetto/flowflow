use crate::application::approvals::ProposalView;
use crate::application::constants::NOTE_ACTION_PROMPT;
use crate::application::i18n::t;
use crate::application::tools::ToolEvent;
use crate::domain::NewTextNote;
use crate::infrastructure::backend::BackendClient;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::action_card::ActionResultCard;
use crate::ui::chat::approval_card::ApprovalCard;
use crate::ui::chat::models::{tool_label, ProposalStatus};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;

fn action_key(note_id: &str) -> String {
    format!("note_action:{note_id}")
}

// One approval card of the current run. The chat keeps its cards as conversation messages so
// they survive a reload; a note has no message table, so a card lives only as long as its run
// and an undecided proposal expires with it.
#[derive(Clone, PartialEq)]
struct Card {
    view: ProposalView,
    status: ProposalStatus,
    edit_error: Option<String>,
}

fn set_status(cards: &mut Signal<Vec<Card>>, id: &str, status: ProposalStatus) {
    if let Some(c) = cards.write().iter_mut().find(|c| c.view.id == id) {
        c.status = status;
        c.edit_error = None;
    }
}

// Drain the run's tool events into the note's live UI state. Returns the sender the agent run
// is given; the loop ends when the run drops it.
fn drain_tool_events(
    mut cards: Signal<Vec<Card>>,
    mut tool_status: Signal<Option<String>>,
    lang: String,
) -> mpsc::UnboundedSender<ToolEvent> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                ToolEvent::Started(name) => {
                    tool_status.set(Some(tool_label(&lang, &name)))
                }
                ToolEvent::Finished(_) => tool_status.set(None),
                ToolEvent::Proposal(view) => cards.write().push(Card {
                    view,
                    status: ProposalStatus::Pending,
                    edit_error: None,
                }),
                ToolEvent::ProposalResolved { id, status } => {
                    set_status(&mut cards, &id, status.into())
                }
                ToolEvent::EditRejected { id, reason } => {
                    if let Some(c) =
                        cards.write().iter_mut().find(|c| c.view.id == id)
                    {
                        c.edit_error = Some(reason);
                    }
                }
            }
        }
    });
    tx
}

// Run-this-note-as-an-action. The note text IS the instruction: it goes through the agent with the
// connected tools (NOTE_ACTION_PROMPT keeps the reply to a one-line confirmation + resource link).
// The result is cached per note so reopening the note shows what it produced instead of re-offering
// a fresh run. Dark unless a backend connector is configured. The run carries a live event
// channel, so it gets the same governed surface as the chat: a connector write holds for the
// approval card below the button instead of being dropped.
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
    let mut cards: Signal<Vec<Card>> = use_signal(Vec::new);
    let mut tool_status: Signal<Option<String>> = use_signal(|| None);

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
                        cards.write().clear();
                        tool_status.set(None);
                        let tx = drain_tool_events(cards, tool_status, lang.clone());
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
                            let outcome = match LlmClient::from_db(&database) {
                                Ok(ai) => {
                                    let ai = Arc::new(ai);
                                    crate::application::chat_surface::prompt_chat_agent(
                                        ai,
                                        NOTE_ACTION_PROMPT,
                                        &c,
                                        Some(tx),
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
                            tool_status.set(None);
                            running.set(false);
                        });
                    },
                    if running() {
                        span { class: "inline-block w-3.5 h-3.5 border-2 border-ios-orange-dark border-t-transparent rounded-full animate-spin" }
                        span { {tool_status().unwrap_or_else(|| running_label.clone())} }
                    } else {
                        span { "{run_label}" }
                    }
                }
                {
                    cards().into_iter().enumerate().map(|(i, card)| {
                        let id = card.view.id.clone();
                        rsx! {
                            ApprovalCard {
                                key: "{i}",
                                view: card.view,
                                status: card.status,
                                edit_error: card.edit_error,
                                on_expired: move |_| set_status(&mut cards, &id, ProposalStatus::Expired),
                                lang: lang.clone(),
                            }
                        }
                    })
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
