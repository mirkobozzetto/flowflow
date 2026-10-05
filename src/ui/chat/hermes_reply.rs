use crate::application::hermes_chat::{self, HermesStep};
use crate::application::i18n::{t, t_args};
use crate::ui::chat::actions::md_to_html;
use crate::ui::icons::{HermesAgentIcon, IconCaretRight};
use crate::ui::AppState;
use dioxus::prelude::*;

/// A Hermes answer: his face, the text, and the tools he used folded into
/// one "N steps" row that opens like "N sources".
#[component]
pub fn HermesReply(
    text: String,
    steps: Vec<HermesStep>,
    #[props(default)] error: Option<String>,
    #[props(default)] live: bool,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    rsx! {
        div {
            class: "flex justify-start items-start gap-2",
            style: "animation: fadeInUp 0.15s ease-out;",
            div { class: "mt-1 shrink-0",
                HermesAgentIcon { size: 20 }
            }
            div { class: "min-w-0 max-w-[85%] py-1",
                if !text.is_empty() {
                    div {
                        class: "text-sm text-stone-900 leading-relaxed break-words prose prose-sm",
                        dangerous_inner_html: md_to_html(&text),
                    }
                }
                if let Some(err) = error {
                    p { class: "text-sm text-ios-red-dark leading-relaxed break-words",
                        {t_args(&lang, "hermes-failed", &[("error", &err)])}
                    }
                }
                if live && text.is_empty() {
                    div { class: "flex items-center gap-1.5 h-6",
                        for delay in ["0s", "0.12s", "0.24s"] {
                            span {
                                class: "w-1.5 h-1.5 rounded-full bg-stone-400",
                                style: "animation: typingDot 1.2s ease-in-out {delay} infinite;",
                            }
                        }
                    }
                }
                if !steps.is_empty() {
                    HermesSteps { steps, lang }
                }
            }
        }
    }
}

#[component]
fn HermesSteps(steps: Vec<HermesStep>, lang: String) -> Element {
    let mut open = use_signal(|| false);
    let label = if steps.len() == 1 {
        t(&lang, "hermes-steps-one")
    } else {
        t_args(
            &lang,
            "hermes-steps-many",
            &[("count", &steps.len().to_string())],
        )
    };
    rsx! {
        div { class: "mt-2 pt-2 border-t border-stone-200/70",
            button {
                class: "pressable w-full min-h-[40px] flex items-center justify-between",
                aria_expanded: "{open()}",
                onclick: move |_| open.set(!open()),
                span { class: "text-xs font-medium text-stone-500", "{label}" }
                span {
                    class: if open() { "text-stone-400 rotate-90 transition-transform duration-180" } else { "text-stone-400 transition-transform duration-180" },
                    IconCaretRight { size: 14 }
                }
            }
            div { class: if open() { "grid grid-rows-[1fr] transition-[grid-template-rows] duration-180" } else { "grid grid-rows-[0fr] transition-[grid-template-rows] duration-180" },
                div { class: "overflow-hidden",
                    div { class: "flex flex-col gap-1.5 mt-1",
                        for (i, (step, n)) in hermes_chat::grouped(&steps).into_iter().enumerate() {
                            div { key: "{i}", class: "flex items-baseline gap-2 text-xs min-w-0",
                                span {
                                    class: if step.failed {
                                        "w-1.5 h-1.5 shrink-0 rounded-full bg-ios-red"
                                    } else if step.running {
                                        "w-1.5 h-1.5 shrink-0 rounded-full bg-stone-400"
                                    } else {
                                        "w-1.5 h-1.5 shrink-0 rounded-full bg-ios-green"
                                    },
                                    style: if step.running { "animation: pulseSoft 1.2s ease-in-out infinite;" } else { "" },
                                }
                                span { class: "shrink-0 font-medium text-stone-700",
                                    {hermes_chat::tool_key(&step.tool).map_or(step.tool.clone(), |k| t(&lang, k))}
                                }
                                span { class: "min-w-0 truncate text-stone-400", "{step.detail}" }
                                if n > 1 {
                                    span { class: "ml-auto shrink-0 text-stone-400 tabular-nums", "×{n}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
