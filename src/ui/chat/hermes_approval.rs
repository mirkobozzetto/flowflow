use crate::application::hermes_chat::{ApprovalStatus, PendingApproval};
use crate::application::i18n::{t, t_args};
use crate::infrastructure::platform::{haptic, haptic_prepare};
use crate::ui::icons::{HermesAgentIcon, IconCheck};
use dioxus::prelude::*;
use std::time::Duration;

/// How long an answer Hermes took stays on its button before the alert goes.
pub const CONFIRM_HOLD: Duration = Duration::from_millis(650);

pub(crate) const CTA: &str = "alert-cta w-full h-[52px] rounded-full text-[17px] font-semibold tracking-[-0.01em] flex items-center justify-center gap-1.5";

#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Primary,
    Secondary,
    Plain,
}

/// Hermes waits on the user's go for a command: an alert over the chat,
/// offering only the choices Hermes allows for it. `answer` is the choice
/// on its way, `done` once Hermes took it. Closing answers nothing; the
/// pill under the reply opens it again.
#[component]
pub fn HermesApprovalAlert(
    approval: PendingApproval,
    total: usize,
    lang: String,
    answer: Option<&'static str>,
    done: bool,
    on_answer: EventHandler<&'static str>,
    on_close: EventHandler<()>,
) -> Element {
    let mut wide = use_signal(|| false);
    let request = approval.request;
    let expired =
        approval.status == ApprovalStatus::Expired && answer.is_none();
    let offered = |c: &str| request.choices.iter().any(|x| x == c);
    let (program, args) = request
        .command
        .split_once(' ')
        .unwrap_or((request.command.as_str(), ""));
    let button = |choice: &'static str, idle: &'static str, tone: Tone| {
        rsx! {
            AlertButton {
                choice,
                idle,
                tone,
                answer,
                done,
                lang: lang.clone(),
                on_pick: move |c| on_answer.call(c),
            }
        }
    };
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center px-6 bg-stone-900/20 backdrop-fade",
            onclick: move |_| {
                if answer.is_none() {
                    on_close.call(());
                }
            },
            div {
                class: "glass-panel w-full max-w-[306px] rounded-[34px] px-[18px] pt-[22px] pb-[18px]",
                style: "animation: popAnchor 0.22s cubic-bezier(0.2, 0.9, 0.25, 1.15);",
                role: "alertdialog",
                aria_modal: "true",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex flex-col items-center text-center",
                    HermesAgentIcon { size: 44 }
                    if expired {
                        p { class: "mt-3 text-[17px] font-semibold text-stone-900",
                            {t(&lang, "hermes-approval-expired-title")}
                        }
                        p { class: "mt-1 text-[13px] leading-snug text-stone-500",
                            {t(&lang, "hermes-approval-expired-body")}
                        }
                    } else {
                        p { class: "mt-3 text-[17px] font-semibold leading-snug text-stone-900",
                            {t(&lang, "hermes-approval-title")}
                            if total > 1 {
                                span { class: "font-medium text-stone-400",
                                    " "
                                    {t_args(&lang, "hermes-approval-count", &[("total", &total.to_string())])}
                                }
                            }
                        }
                        if !request.description.is_empty() {
                            p { class: "mt-1 text-[13px] leading-snug text-stone-500",
                                "{request.description}"
                            }
                        }
                    }
                }
                button {
                    class: if wide() { "mt-3 w-full text-left rounded-[18px] bg-stone-900/5 px-3.5 py-2.5 font-mono text-[12px] leading-[1.5] break-all" } else { "mt-3 w-full text-left rounded-[18px] bg-stone-900/5 px-3.5 py-2.5 font-mono text-[12px] leading-[1.5] break-all line-clamp-3" },
                    onclick: move |_| wide.set(!wide()),
                    span { class: if expired { "font-semibold text-stone-400" } else { "font-semibold text-stone-900" },
                        "{program}"
                    }
                    span { class: if expired { "text-stone-400" } else { "text-stone-600" }, " {args}" }
                }
                if expired {
                    button {
                        class: "{CTA} mt-4 bg-stone-900/5 text-stone-900",
                        onclick: move |_| on_close.call(()),
                        {t(&lang, "hermes-approval-ok")}
                    }
                } else {
                    div { class: "mt-4 flex flex-col gap-2",
                        if offered("once") {
                            {button("once", "hermes-approval-run", Tone::Primary)}
                        }
                        if offered("session") {
                            {button("session", "hermes-approval-session", Tone::Secondary)}
                        }
                        if offered("deny") {
                            {button("deny", "hermes-approval-deny", Tone::Plain)}
                        }
                    }
                }
            }
        }
    }
}

// One choice: idle, on its way ("Exécution…"), or taken ("✓ Exécutée");
// while one choice is out, the others step back.
#[component]
fn AlertButton(
    choice: &'static str,
    idle: &'static str,
    tone: Tone,
    answer: Option<&'static str>,
    done: bool,
    lang: String,
    on_pick: EventHandler<&'static str>,
) -> Element {
    let mine = answer == Some(choice);
    let look = match tone {
        Tone::Primary => "bg-ios-orange text-white",
        Tone::Secondary => "bg-stone-900/5 text-stone-900 !text-[15px]",
        Tone::Plain => "text-stone-600 !font-medium",
    };
    let fade = if answer.is_some() && !mine {
        "opacity-30"
    } else {
        ""
    };
    let busy = if choice == "deny" {
        "hermes-approval-denying"
    } else {
        "hermes-approval-running"
    };
    rsx! {
        button {
            class: "{CTA} {look} {fade}",
            disabled: answer.is_some(),
            "data-done": if mine && done { "1" },
            onpointerdown: move |_| haptic_prepare("light"),
            onclick: move |_| {
                haptic("light");
                on_pick.call(choice);
            },
            if mine && done {
                IconCheck { size: 18 }
                {t(&lang, &format!("hermes-receipt-{choice}"))}
            } else if mine {
                {t(&lang, busy)}
            } else {
                {t(&lang, idle)}
            }
        }
    }
}

/// Under the live reply while its alert is closed: opens it again.
#[component]
pub fn HermesApprovalPill(
    expired: bool,
    lang: String,
    onclick: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            class: "pressable min-h-[44px] flex items-center",
            onclick: move |_| onclick.call(()),
            span { class: if expired { "h-9 pl-2.5 pr-3 inline-flex items-center gap-2 rounded-full bg-stone-900/5 text-xs font-medium text-stone-500" } else { "h-9 pl-2.5 pr-3 inline-flex items-center gap-2 rounded-full bg-ios-orange/10 text-xs font-medium text-ios-orange-dark" },
                span {
                    class: if expired { "w-1.5 h-1.5 rounded-full bg-stone-300" } else { "w-1.5 h-1.5 rounded-full bg-ios-orange" },
                    style: if expired { "" } else { "animation: pulseSoft 1.2s ease-in-out infinite;" },
                }
                {t(&lang, if expired { "hermes-approval-expired" } else { "hermes-approval-waiting" })}
            }
        }
    }
}
