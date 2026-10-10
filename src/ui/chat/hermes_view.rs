use crate::application::hermes_chat::{
    self, ApprovalStatus, HermesError, HermesTurn, LiveReply, PendingApproval,
};
use crate::application::hermes_message::{self, Asked, Attachment};
use crate::application::i18n::{t, t_args};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::haptic;
use crate::ui::chat::empty_state::ChatEmptyState;
use crate::ui::chat::hermes_approval::{
    HermesApprovalAlert, HermesApprovalPill, CONFIRM_HOLD,
};
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
        HermesError::ApprovalGone => t(lang, "hermes-approval-expired"),
        HermesError::Server(msg) => crate::application::i18n::t_args(
            lang,
            "hermes-failed",
            &[("error", msg)],
        ),
    }
}

// "Opus 5.5 · 99 skills": what this Hermes runs on and what it can draw on;
// a count Hermes cannot serve is left out.
fn facts_line(lang: &str, app: AppState) -> Option<String> {
    let options = (app.hermes_models)()?;
    let model = (app.hermes_pick)().map(|(_, m)| m).unwrap_or(options.model);
    let mut parts = vec![hermes_chat::model_label(&model)];
    let skills = (app.hermes_skills)().and_then(|s| s.ok()).map(|s| s.len());
    if let Some(n) = skills.filter(|n| *n > 0) {
        parts.push(t_args(
            lang,
            "hermes-facts-skills",
            &[("count", &n.to_string())],
        ));
    }
    Some(parts.join(" · "))
}

// The answer on its way, kept on screen until Hermes took it and a beat after.
#[derive(Clone, PartialEq)]
struct Answering {
    approval: PendingApproval,
    total: usize,
    choice: &'static str,
    done: bool,
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
    // The run the live reply follows, and the request whose alert the user
    // closed without answering.
    let mut run: Signal<Option<String>> = use_signal(|| None);
    let mut closed: Signal<Option<String>> = use_signal(|| None);
    let mut answering: Signal<Option<Answering>> = use_signal(|| None);
    // Scheduled tasks on Hermes, for the empty conversation.
    let mut jobs: Signal<Option<usize>> = use_signal(|| None);
    // The skills, read once per opening of the Hermes chat: the "+" menu,
    // "/" and the suggested skills all draw on them.
    use_hook(move || {
        app.hermes_skills.set(None);
        spawn(async move {
            app.hermes_skills
                .set(Some(hermes_chat::skills(&db()).await));
        });
    });
    let input = use_signal(String::new);
    let pending_audio: Signal<Option<(String, f64)>> = use_signal(|| None);

    // Follows a run to its end, then trades the live reply for the stored
    // history. A failed run keeps its reply on screen with the reason.
    let mut track = move |sid: String, run_id: String| {
        live.set(Some(LiveReply::default()));
        run.set(Some(run_id.clone()));
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
            if let Ok(mut fresh) = hermes_chat::history(&database, &sid).await {
                if let Some(reply) = live.peek().as_ref() {
                    hermes_chat::keep_receipts(&mut fresh, reply);
                }
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
        let slot = sid.as_deref().unwrap_or(hermes_chat::LAST_PICK);
        app.hermes_pick
            .set(hermes_chat::chosen_model(&db.peek(), slot));
        app.hermes_effort
            .set(hermes_chat::chosen_effort(&db.peek(), slot));
        spawn(async move {
            let database = db();
            let options = hermes_chat::model_options(&database).await;
            let Some(sid) = sid else {
                // A new conversation: the options read doubles as the
                // reachability check, the counts fill the empty screen.
                match options {
                    Ok(o) => app.hermes_models.set(Some(o)),
                    Err(e) => {
                        problem.set(Some(e));
                        return;
                    }
                }
                jobs.set(hermes_chat::jobs_count(&database).await);
                return;
            };
            app.hermes_models.set(options.ok());
            match hermes_chat::history(&database, &sid).await {
                Ok(history) => turns.set(history),
                Err(e) => {
                    problem.set(Some(e));
                    return;
                }
            }
            if let Some((run_id, message)) =
                hermes_chat::pending(&database, &sid)
            {
                let asked = hermes_message::split(&message);
                let last = turns.peek().iter().rev().find_map(|t| match t {
                    HermesTurn::User(a) => Some(a.question.clone()),
                    _ => None,
                });
                if last.as_deref() != Some(asked.question.as_str()) {
                    turns.write().push(HermesTurn::User(asked));
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

    // One tap sends one answer; what Hermes makes of it lands on the reply.
    // The alert stays on its own copy of the request until the answer has
    // shown on its button, whatever the stream does meanwhile.
    let mut answer = move |approval: PendingApproval,
                           total: usize,
                           choice: &'static str| {
        let Some(run_id) = run.peek().clone() else {
            return;
        };
        let id = approval.request.request_id.clone();
        if answering.peek().is_some()
            || !live.write().as_mut().is_some_and(|r| r.sending(&id))
        {
            return;
        }
        answering.set(Some(Answering {
            approval,
            total,
            choice,
            done: false,
        }));
        spawn(async move {
            let result = hermes_chat::answer(&db(), &run_id, &id, choice).await;
            if let Some(reply) = live.write().as_mut() {
                reply.answered(&id, choice, &result);
            }
            if result.is_ok() {
                if let Some(a) = answering.write().as_mut() {
                    a.done = true;
                }
                haptic("soft");
                futures_timer::Delay::new(CONFIRM_HOLD).await;
            }
            answering.set(None);
        });
    };
    let mut acknowledge = move |request_id: String| {
        haptic("light");
        if let Some(reply) = live.write().as_mut() {
            reply.acknowledge(&request_id);
        }
    };
    // The first request still on screen, with how many wait in all.
    let asking = live().filter(|r| !r.done).and_then(|r| {
        r.approvals.first().cloned().map(|a| (a, r.approvals.len()))
    });
    let alert_open = asking.as_ref().is_some_and(|(a, _)| {
        closed().as_deref() != Some(a.request.request_id.as_str())
    });
    let alert = answering()
        .map(|a| (a.approval, a.total, Some(a.choice), a.done))
        .or_else(|| {
            asking
                .clone()
                .filter(|_| alert_open)
                .map(|(a, total)| (a, total, None, false))
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
                    ChatEmptyState {
                        hermes: true,
                        facts: facts_line(&lang, app),
                        jobs: jobs().filter(|n| *n > 0).map(|n| t_args(&lang, "hermes-facts-jobs", &[("count", &n.to_string())])),
                        on_jobs: move |_| crate::ui::hermes_jobs::open(app),
                    }
                } else {
                    div { class: "space-y-3",
                        for (i, turn) in turns().into_iter().enumerate() {
                            match turn {
                                HermesTurn::User(a) => rsx! {
                                    div { key: "{i}", UserBubble { text: a.question, ink: true, attachments: a.attachments, skills: a.skills } }
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
                        if let Some((a, _)) = asking.clone().filter(|_| alert.is_none()) {
                            div { class: "pl-7 -mt-1",
                                HermesApprovalPill {
                                    expired: a.status == ApprovalStatus::Expired,
                                    lang: lang.clone(),
                                    onclick: move |_| closed.set(None),
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some((a, total, sent, done)) = alert {
            HermesApprovalAlert {
                key: "{a.request.request_id}",
                approval: a.clone(),
                total,
                lang: lang.clone(),
                answer: sent,
                done,
                on_answer: {
                    let a = a.clone();
                    move |choice: &'static str| answer(a.clone(), total, choice)
                },
                on_close: {
                    let id = a.request.request_id.clone();
                    let expired = a.status == ApprovalStatus::Expired;
                    move |_| {
                        if expired {
                            acknowledge(id.clone());
                        } else {
                            closed.set(Some(id.clone()));
                        }
                    }
                },
            }
        }
        Composer {
            role: ComposerRole::AskHermes,
            input: input,
            disabled: busy,
            pending_audio: pending_audio,
            attachments: app.hermes_attachments,
            on_commit: move |q: String| {
                if live.peek().as_ref().is_some_and(|r| !r.done) {
                    return;
                }
                let sent: Vec<Attachment> = app.hermes_attachments.take();
                problem.set(None);
                let (skills, others): (Vec<&Attachment>, Vec<&Attachment>) =
                    sent.iter().partition(|a| matches!(a, Attachment::Skill { .. }));
                let names = |list: Vec<&Attachment>| list.iter().map(|a| a.label().to_string()).collect();
                turns.write().push(HermesTurn::User(Asked {
                    question: q.clone(),
                    attachments: names(others),
                    skills: names(skills),
                }));
                live.set(Some(LiveReply::default()));
                spawn(async move {
                    let database = db();
                    let current = session.peek().clone();
                    let pick = app.hermes_pick.peek().clone();
                    let (provider, model) = pick.clone()
                        .or_else(|| app.hermes_models.peek().as_ref().map(|o| (o.provider.clone(), o.model.clone())))
                        .unwrap_or_default();
                    let effort = hermes_chat::effective_effort(
                        app.hermes_effort.peek().as_deref(),
                        &provider,
                        &model,
                    );
                    match hermes_chat::send(&database, current.clone(), &q, &sent, pick, effort).await {
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
