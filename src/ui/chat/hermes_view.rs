use crate::application::hermes_chat::{
    self, HermesError, HermesTurn, LiveReply,
};
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::empty_state::ChatEmptyState;
use crate::ui::chat::hermes_reply::HermesReply;
use crate::ui::chat::user_bubble::UserBubble;
use crate::ui::composer::{Composer, ComposerRole};
use crate::ui::state::{SettingsSection, View};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

/// One sentence for what keeps Hermes from answering.
pub(crate) fn problem_text(lang: &str, e: &HermesError) -> String {
    match e {
        HermesError::NotConfigured => t(lang, "hermes-not-configured"),
        HermesError::Unreachable => t(lang, "hermes-unreachable"),
        HermesError::KeyRefused => t(lang, "hermes-key-refused"),
        HermesError::NotFound => t(lang, "hermes-unreachable"),
        HermesError::Server(msg) => crate::application::i18n::t_args(
            lang,
            "hermes-failed",
            &[("error", msg)],
        ),
    }
}

fn view_session(view: &View) -> Option<Option<String>> {
    match view {
        View::HermesChat { session_id } => Some(session_id.clone()),
        _ => None,
    }
}

#[component]
pub fn HermesChatView() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();

    let mut session: Signal<Option<String>> = use_signal(|| None);
    let mut loaded = use_signal(|| false);
    let mut turns: Signal<Vec<HermesTurn>> = use_signal(Vec::new);
    let mut live: Signal<Option<LiveReply>> = use_signal(|| None);
    let mut problem: Signal<Option<HermesError>> = use_signal(|| None);
    let input = use_signal(String::new);
    let pending_audio: Signal<Option<(String, f64)>> = use_signal(|| None);

    // Follows a run to its end, then trades the live reply for the stored
    // history. A failed run keeps its reply on screen with the reason.
    let mut track = move |sid: String, run_id: String| {
        live.set(Some(LiveReply::default()));
        spawn(async move {
            let database = db();
            let followed =
                hermes_chat::follow(&database, &sid, &run_id, |event| {
                    if let Some(reply) = live.write().as_mut() {
                        reply.apply(event);
                    }
                })
                .await;
            app.invalidate_data();
            if let Err(e) = followed {
                problem.set(Some(e));
                return;
            }
            if live.peek().as_ref().is_some_and(|r| r.error.is_some()) {
                return;
            }
            if let Ok(fresh) = hermes_chat::history(&database, &sid).await {
                turns.set(fresh);
                live.set(None);
            }
        });
    };

    // Load on mount and whenever the Chats list opens another conversation;
    // the session this view just created itself is already on screen.
    use_effect(move || {
        let Some(sid) = view_session(&(app.view)()) else {
            return;
        };
        if *loaded.peek() && sid == *session.peek() {
            return;
        }
        loaded.set(true);
        session.set(sid.clone());
        turns.set(Vec::new());
        live.set(None);
        problem.set(None);
        spawn(async move {
            let database = db();
            let Some(sid) = sid else {
                if let Err(e) = hermes_chat::ready(&database).await {
                    problem.set(Some(e));
                }
                return;
            };
            match hermes_chat::history(&database, &sid).await {
                Ok(history) => turns.set(history),
                Err(e) => {
                    problem.set(Some(e));
                    return;
                }
            }
            if let Some((run_id, question)) =
                hermes_chat::pending(&database, &sid)
            {
                let asked = turns.peek().iter().rev().find_map(|t| match t {
                    HermesTurn::User(q) => Some(q.clone()),
                    _ => None,
                });
                if asked.as_deref() != Some(question.as_str()) {
                    turns.write().push(HermesTurn::User(question));
                }
                track(sid, run_id);
            }
        });
    });

    use_effect(move || {
        let _ = turns().len();
        let _ = live().map(|r| (r.text.len(), r.steps.len()));
        dioxus::document::eval(
            "let el = document.getElementById('chat-messages'); if (el) el.scrollTop = el.scrollHeight;",
        );
    });

    let busy = live().is_some_and(|r| !r.done);
    let is_empty = turns().is_empty() && live().is_none();

    rsx! {
        div {
            class: "overflow-hidden relative",
            style: "height: calc(100% - var(--keyboard-inset, 0px));",
            div { id: "chat-messages", class: "h-full overflow-y-auto px-4 pt-4 safe-pb-40 lg:px-[max(1rem,calc((100%-48rem)/2))]",
                if let Some(e) = problem() {
                    div { class: "rounded-xl border border-stone-200 bg-warm-white p-4 mb-3 space-y-3",
                        p { class: "text-sm text-stone-700 leading-relaxed", {problem_text(&lang, &e)} }
                        button {
                            class: crate::ui::kit::PILL_PRIMARY,
                            onclick: move |_| {
                                app.previous_view.set(Some((app.view)()));
                                app.view.set(View::SettingsSection(SettingsSection::Connections));
                            },
                            {t(&lang, "hermes-open-settings")}
                        }
                    }
                }
                if is_empty && problem().is_none() {
                    ChatEmptyState { hermes: true }
                } else {
                    div { class: "space-y-3",
                        for (i, turn) in turns().into_iter().enumerate() {
                            match turn {
                                HermesTurn::User(text) => rsx! {
                                    div { key: "{i}", UserBubble { text, ink: true } }
                                },
                                HermesTurn::Reply { text, steps } => rsx! {
                                    div { key: "{i}", HermesReply { text, steps } }
                                },
                            }
                        }
                        if let Some(reply) = live() {
                            HermesReply {
                                text: reply.text,
                                steps: reply.steps,
                                error: reply.error,
                                live: !reply.done,
                            }
                        }
                    }
                }
            }
        }
        Composer {
            role: ComposerRole::AskHermes,
            input: input,
            disabled: busy,
            pending_audio: pending_audio,
            on_commit: move |q: String| {
                if live.peek().as_ref().is_some_and(|r| !r.done) {
                    return;
                }
                problem.set(None);
                turns.write().push(HermesTurn::User(q.clone()));
                live.set(Some(LiveReply::default()));
                spawn(async move {
                    let database = db();
                    let current = session.peek().clone();
                    match hermes_chat::send(&database, current.clone(), &q).await {
                        Ok((sid, run_id)) => {
                            if current.is_none() {
                                session.set(Some(sid.clone()));
                                app.view.set(View::HermesChat {
                                    session_id: Some(sid.clone()),
                                });
                                app.invalidate_data();
                            }
                            track(sid, run_id);
                        }
                        Err(e) => {
                            live.set(None);
                            problem.set(Some(e));
                        }
                    }
                });
            },
        }
    }
}
