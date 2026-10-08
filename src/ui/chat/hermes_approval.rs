use crate::application::hermes_chat::{ApprovalStatus, PendingApproval};
use crate::application::i18n::{t, t_args};
use crate::infrastructure::platform::haptic;
use crate::ui::icons::HermesAgentIcon;
use dioxus::prelude::*;

const CTA: &str = "pressable w-full h-[52px] rounded-full text-[17px] font-semibold tracking-[-0.01em] active:scale-[0.97] transition-transform disabled:opacity-55";

/// Hermes waits on the user's go for a command: an alert over the chat,
/// offering only the choices Hermes allows for it. Closing it answers
/// nothing; the pill under the reply opens it again.
#[component]
pub fn HermesApprovalAlert(
    approval: PendingApproval,
    total: usize,
    lang: String,
    on_answer: EventHandler<&'static str>,
    on_close: EventHandler<()>,
) -> Element {
    let mut wide = use_signal(|| false);
    let mut chosen: Signal<Option<&'static str>> = use_signal(|| None);
    let request = approval.request.clone();
    let sending = approval.status == ApprovalStatus::Sending;
    let expired = approval.status == ApprovalStatus::Expired;
    let offered = |c: &str| request.choices.iter().any(|x| x == c);
    let (program, args) = request
        .command
        .split_once(' ')
        .unwrap_or((request.command.as_str(), ""));
    let mut pick = move |choice: &'static str| {
        haptic("light");
        chosen.set(Some(choice));
        on_answer.call(choice);
    };
    let label = |choice: &str, idle: &str, busy: &str| {
        t(
            &lang,
            if sending && chosen() == Some(choice) {
                busy
            } else {
                idle
            },
        )
    };
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center px-6 bg-stone-900/20 backdrop-fade",
            onclick: move |_| {
                if !sending {
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
                            button {
                                class: "{CTA} bg-ios-orange text-white",
                                disabled: sending,
                                onclick: move |_| pick("once"),
                                {label("once", "hermes-approval-run", "hermes-approval-running")}
                            }
                        }
                        if offered("session") {
                            button {
                                class: "{CTA} bg-stone-900/5 text-stone-900 !text-[15px]",
                                disabled: sending,
                                onclick: move |_| pick("session"),
                                {label("session", "hermes-approval-session", "hermes-approval-running")}
                            }
                        }
                        if offered("deny") {
                            button {
                                class: "{CTA} text-stone-600 !font-medium",
                                disabled: sending,
                                onclick: move |_| pick("deny"),
                                {label("deny", "hermes-approval-deny", "hermes-approval-denying")}
                            }
                        }
                    }
                }
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
