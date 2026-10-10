use crate::application::hermes_jobs::{self, JobAction};
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::platform::{haptic, haptic_prepare};
use crate::ui::chat::hermes_approval::{CONFIRM_HOLD, CTA};
use crate::ui::chat::hermes_problem_text;
use crate::ui::icons::{HermesAgentIcon, IconCheck};
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

// The alert of Hermes' approvals: Hermes' face over the dimmed screen; a tap
// outside closes it, unless an answer is on its way.
#[component]
fn AlertFrame(
    locked: bool,
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center px-6 bg-stone-900/20 backdrop-fade",
            onclick: move |_| {
                if !locked {
                    on_close.call(());
                }
            },
            div {
                class: "glass-panel w-full max-w-[306px] rounded-[34px] px-[18px] pt-[22px] pb-[18px]",
                style: "animation: popAnchor 0.22s cubic-bezier(0.2, 0.9, 0.25, 1.15);",
                role: "alertdialog",
                aria_modal: "true",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex justify-center", HermesAgentIcon { size: 44 } }
                {children}
            }
        }
    }
}

/// The task, as an alert recalls it.
#[component]
fn JobWell(name: String, lines: Vec<String>) -> Element {
    rsx! {
        div { class: "mt-3 w-full rounded-[18px] bg-stone-900/5 px-3.5 py-2.5 text-[13px] leading-[1.5] text-stone-600",
            span { class: "block font-semibold text-stone-900", "{name}" }
            for (i, line) in lines.into_iter().enumerate() {
                span { key: "{i}", class: "block", "{line}" }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum Phase {
    Asking,
    Sending,
    Done,
    Failed(String),
}

fn key(action: JobAction) -> &'static str {
    match action {
        JobAction::RunNow => "run",
        JobAction::Pause => "pause",
        JobAction::Resume => "resume",
        JobAction::Delete => "delete",
    }
}

/// One action on a task, asked first. The answer lands on its own button,
/// the list is read again, then the alert goes.
#[component]
pub fn JobAlert(
    job_id: String,
    name: String,
    schedule: String,
    action: JobAction,
    on_close: EventHandler<()>,
) -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let mut phase = use_signal(|| Phase::Asking);
    let k = key(action);
    let busy = matches!(phase(), Phase::Sending | Phase::Done);
    let tone = if action == JobAction::Delete {
        "bg-ios-red text-white"
    } else {
        "bg-ios-orange text-white"
    };
    let cancel = if busy {
        format!("{CTA} text-stone-600 !font-medium opacity-30")
    } else {
        format!("{CTA} text-stone-600 !font-medium")
    };
    let line = match phase() {
        Phase::Failed(err) => err,
        _ => t(&lang, &format!("hermes-job-{k}-body")),
    };
    let failed = matches!(phase(), Phase::Failed(_));
    let go = {
        let lang = lang.clone();
        move |_| {
            haptic("light");
            phase.set(Phase::Sending);
            let (id, lang) = (job_id.clone(), lang.clone());
            spawn(async move {
                let database = db();
                if let Err(e) = hermes_jobs::act(&database, &id, action).await {
                    phase.set(Phase::Failed(hermes_problem_text(&lang, &e)));
                    return;
                }
                phase.set(Phase::Done);
                haptic("soft");
                super::reload(app, &database).await;
                futures_timer::Delay::new(CONFIRM_HOLD).await;
                on_close.call(());
                // A deleted task's screen has nothing left to show.
                if action == JobAction::Delete
                    && matches!((app.view)(), View::HermesJob { .. })
                {
                    app.view.set(View::HermesJobs);
                }
            });
        }
    };
    rsx! {
        AlertFrame { locked: busy, on_close,
            p { class: "mt-3 text-center text-[17px] font-semibold leading-snug text-stone-900",
                {t(&lang, &format!("hermes-job-{k}-title"))}
            }
            p {
                class: if failed { "mt-1 text-center text-[13px] leading-snug text-ios-red-dark" } else { "mt-1 text-center text-[13px] leading-snug text-stone-500" },
                "{line}"
            }
            JobWell { name, lines: vec![schedule] }
            div { class: "mt-4 flex flex-col gap-2",
                button {
                    class: "{CTA} {tone}",
                    disabled: busy,
                    "data-done": (phase() == Phase::Done).then_some("1"),
                    onpointerdown: move |_| haptic_prepare("light"),
                    onclick: go,
                    match phase() {
                        Phase::Done => rsx! {
                            IconCheck { size: 18 }
                            {t(&lang, &format!("hermes-job-{k}-done"))}
                        },
                        Phase::Sending => rsx! { {t(&lang, &format!("hermes-job-{k}-busy"))} },
                        _ => rsx! { {t(&lang, &format!("hermes-job-{k}-go"))} },
                    }
                }
                button {
                    class: "{cancel}",
                    disabled: busy,
                    onclick: move |_| on_close.call(()),
                    {t(&lang, "hermes-job-cancel")}
                }
            }
        }
    }
}

/// What a new task will do, once Hermes has it.
#[component]
pub fn CreatedAlert(
    name: String,
    lines: Vec<String>,
    on_close: EventHandler<()>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    rsx! {
        AlertFrame { locked: false, on_close,
            p { class: "mt-3 text-center text-[17px] font-semibold leading-snug text-stone-900",
                {t(&lang, "hermes-job-created")}
            }
            p { class: "mt-1 text-center text-[13px] leading-snug text-stone-500",
                {t(&lang, "hermes-job-created-body")}
            }
            JobWell { name, lines }
            button {
                class: "{CTA} mt-4 bg-stone-900/5 text-stone-900",
                onclick: move |_| on_close.call(()),
                {t(&lang, "hermes-approval-ok")}
            }
        }
    }
}
