use crate::application::hermes_chat::{self, HermesStep};
use crate::application::i18n::{t, t_args};
use crate::ui::chat::code_block::CodeBlock;
use crate::ui::chat::markdown::{md_blocks, MdBlock};
use crate::ui::icons::{HermesAgentIcon, IconCaretRight, IconFileArrowUp};
use crate::ui::AppState;
use dioxus::prelude::*;

/// A Hermes answer: his face, the text, and the tools he used folded into
/// one "N steps" row that opens like "N sources". While he works the row
/// stays open, with what he says before each tool.
#[component]
pub fn HermesReply(
    text: String,
    steps: Vec<HermesStep>,
    #[props(default)] error: Option<String>,
    #[props(default)] live: bool,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let stopped = steps
        .last()
        .is_some_and(|s| s.tool == hermes_chat::STOPPED_TOOL);
    let has_text = !text.is_empty();
    rsx! {
        div {
            class: "flex justify-start items-start gap-2",
            style: "animation: fadeInUp 0.15s ease-out;",
            div { class: "mt-1 shrink-0",
                HermesAgentIcon { size: 20 }
            }
            // No bubble around Hermes: the reply takes the whole width, so a
            // wide table does too.
            div { class: "min-w-0 flex-1 py-1",
                if has_text {
                    div { class: "text-sm text-stone-900 leading-relaxed break-words prose prose-sm",
                        for (i, block) in md_blocks(&text).into_iter().enumerate() {
                            match block {
                                MdBlock::Html(html) => rsx! {
                                    div { key: "{i}", dangerous_inner_html: html }
                                },
                                MdBlock::Code { lang, code } => rsx! {
                                    CodeBlock { key: "{i}", lang, code }
                                },
                                MdBlock::File { name, kind } => rsx! {
                                    div {
                                        key: "{i}",
                                        class: "bg-warm-white border border-stone-200 rounded-xl h-[60px] flex items-center px-3 my-2",
                                        div { class: "w-9 h-9 rounded-lg bg-warm-white border border-ios-orange/20 flex items-center justify-center flex-shrink-0 text-ios-orange-dark",
                                            IconFileArrowUp { size: 18 }
                                        }
                                        div { class: "flex-1 min-w-0 ml-3",
                                            div { class: "text-sm font-medium text-stone-900 truncate leading-tight", "{name}" }
                                            div { class: "text-xs text-stone-400 leading-tight",
                                                {t_args(&lang, "hermes-file-kind", &[("kind", &kind)])}
                                            }
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
                if stopped {
                    p { class: "text-xs text-stone-500 leading-relaxed", {t(&lang, "hermes-stopped")} }
                }
                if let Some(err) = error {
                    p { class: "text-sm text-ios-red-dark leading-relaxed break-words",
                        {t_args(&lang, "hermes-failed", &[("error", &err)])}
                    }
                }
                if live && !has_text && steps.is_empty() {
                    div { class: "flex items-center gap-1.5 h-6",
                        for delay in ["0s", "0.12s", "0.24s"] {
                            span {
                                class: "w-1.5 h-1.5 rounded-full bg-stone-400",
                                style: "animation: typingDot 1.2s ease-in-out {delay} infinite;",
                            }
                        }
                    }
                }
                if steps.iter().any(|s| s.tool != hermes_chat::STOPPED_TOOL) {
                    HermesSteps { steps, lang, live, divided: has_text || stopped }
                }
            }
        }
    }
}

#[component]
fn HermesSteps(
    steps: Vec<HermesStep>,
    lang: String,
    live: bool,
    divided: bool,
) -> Element {
    let mut open = use_signal(|| false);
    // Hermes' lines and the person's guidance are no steps of their own.
    let count = steps
        .iter()
        .filter(|s| {
            !hermes_chat::is_note(s) && s.tool != hermes_chat::STOPPED_TOOL
        })
        .count();
    let label = if count == 1 {
        t(&lang, "hermes-steps-one")
    } else {
        t_args(&lang, "hermes-steps-many", &[("count", &count.to_string())])
    };
    let shown = live || open();
    let stopped = steps
        .last()
        .is_some_and(|s| s.tool == hermes_chat::STOPPED_TOOL);
    rsx! {
        div { class: if divided { "mt-2 pt-2 border-t border-stone-200/70" } else { "" },
            if count > 0 {
                button {
                    class: "pressable w-full min-h-[40px] flex items-center justify-between",
                    aria_expanded: "{shown}",
                    onclick: move |_| open.set(!open()),
                    span { class: "text-xs font-medium text-stone-500", "{label}" }
                    span {
                        class: if shown { "text-stone-400 rotate-90 transition-transform duration-180" } else { "text-stone-400 transition-transform duration-180" },
                        IconCaretRight { size: 14 }
                    }
                }
            }
            div { class: if shown { "grid grid-rows-[1fr] transition-[grid-template-rows] duration-180" } else { "grid grid-rows-[0fr] transition-[grid-template-rows] duration-180" },
                div { class: "overflow-hidden",
                    div { class: "flex flex-col gap-1.5 mt-1 pb-1",
                        for (i, (step, n)) in hermes_chat::grouped(&steps).into_iter().enumerate() {
                            if step.tool == hermes_chat::INTERIM_TOOL {
                                p { key: "{i}", class: "text-xs text-stone-600 italic leading-snug break-words",
                                    "{step.detail}"
                                }
                            } else if step.tool == hermes_chat::STEER_TOOL {
                                div { key: "{i}", class: "flex items-baseline gap-2 text-xs min-w-0",
                                    span { class: "w-1.5 h-1.5 shrink-0 rounded-full bg-ios-orange" }
                                    span { class: "shrink-0 font-medium text-stone-700", {t(&lang, "hermes-step-steer")} }
                                    span { class: "min-w-0 truncate text-stone-400", "{step.detail}" }
                                }
                            } else if step.tool != hermes_chat::STOPPED_TOOL {
                                div { key: "{i}", class: "flex items-baseline gap-2 text-xs min-w-0",
                                    span {
                                        // A command the user kept from running, or cut by
                                        // stopping the run, is no failure.
                                        class: if step.failed && (stopped || step.tool.starts_with(hermes_chat::RECEIPT_TOOL)) {
                                            "w-1.5 h-1.5 shrink-0 rounded-full bg-stone-300"
                                        } else if step.failed {
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
}
