use crate::application::i18n::t_args;
use crate::application::rag::{RagTrace, StepOutcome};
use crate::ui::AppState;
use dioxus::prelude::*;

/// Debug-only pipeline trace under a bot answer: a compact "N steps - X.Xs" line
/// that expands into the per-step table. Rendered only when the Settings debug
/// toggle is on; end users never see it.
#[component]
pub fn TraceAccordion(trace: RagTrace) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut open = use_signal(|| false);

    let (steps, secs) = trace.summary();
    let label = t_args(
        &lang,
        "chat-trace-summary",
        &[
            ("steps", &steps.to_string()),
            ("secs", &format!("{secs:.1}")),
        ],
    );

    rsx! {
        div { class: "mt-1.5 border-t border-stone-100 pt-1.5",
            button {
                class: "text-xs text-stone-400 active:text-stone-600 hover:text-stone-600 transition-colors duration-150",
                onclick: move |_| open.set(!open()),
                "{label}"
            }
            if open() {
                div { class: "mt-1 space-y-0.5",
                    for step in trace.steps.iter() {
                        div { class: "flex items-baseline gap-2 text-xs font-mono",
                            span { class: "w-16 shrink-0 text-stone-600", "{step.step}" }
                            span {
                                class: match step.outcome {
                                    StepOutcome::Ok => "text-ios-green",
                                    StepOutcome::Fallback => "text-ios-red",
                                    StepOutcome::Skipped => "text-stone-400",
                                },
                                match step.outcome {
                                    StepOutcome::Ok => "ok",
                                    StepOutcome::Fallback => "fallback",
                                    StepOutcome::Skipped => "skip",
                                }
                            }
                            span { class: "text-stone-500", "{step.latency_ms}ms" }
                            if let Some(ref model) = step.model {
                                span { class: "text-stone-400 truncate", "{model}" }
                            }
                            if let Some(tin) = step.tokens_in {
                                span { class: "text-stone-400", "~{tin}t in" }
                            }
                            if let Some(tout) = step.tokens_out {
                                span { class: "text-stone-400", "~{tout}t out" }
                            }
                        }
                    }
                }
            }
        }
    }
}
