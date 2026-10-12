use super::menu::JobMenuItems;
use crate::application::hermes_delivery::delivers_to_flowflow;
use crate::application::hermes_jobs::{
    next_run_text, schedule_text, status_text, JobAction,
};
use crate::application::i18n::{t, t_args};
use crate::infrastructure::hermes::HermesJob;
use crate::ui::icons::{IconBell, IconDotsThree};
use crate::ui::kit;
use crate::ui::AppState;
use dioxus::prelude::*;

/// Height of the actions panel (three rows and a line), and the gap kept to
/// the bottom of the screen: a card low on screen opens it higher.
const MENU_H: u32 = 150;
const EDGE: u32 = 12;

/// How the last run went: a dot and a word.
#[component]
pub fn JobStatus(job: HermesJob, lang: String) -> Element {
    let (dot, ink) = match job.last_status.as_deref() {
        None => ("bg-stone-300", "text-stone-500"),
        Some("ok") => ("bg-ios-green", "text-stone-500"),
        Some(_) => ("bg-ios-red", "text-ios-red-dark"),
    };
    rsx! {
        span { class: "inline-flex items-center gap-1.5 {ink}",
            span { class: "w-1.5 h-1.5 rounded-full {dot}" }
            {status_text(&lang, &job)}
        }
    }
}

/// One task in the list: its name, when it runs on the phone's clock, its
/// next run and how the last one went; "…" opens its actions.
#[component]
pub fn JobCard(
    job: HermesJob,
    shift: i32,
    menu_open: bool,
    on_open: EventHandler<()>,
    on_menu: EventHandler<()>,
    on_pick: EventHandler<JobAction>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let paused = job.paused();
    let next = next_run_text(&lang, &job, &chrono::Local::now());
    // Where "…" was tapped: the panel opens under it, never past the bottom.
    let mut tapped_y = use_signal(|| 0.0f64);
    rsx! {
        div {
            class: "relative bg-warm-white pl-4 pr-1 py-3 border border-stone-200 rounded-xl mb-2.5 cursor-pointer flex items-start gap-1",
            onclick: move |_| on_open.call(()),
            div { class: if paused { "flex-1 min-w-0 pt-0.5 opacity-60" } else { "flex-1 min-w-0 pt-0.5" },
                h3 { class: "flex items-center gap-1.5 min-w-0",
                    span { class: "truncate font-semibold text-base tracking-[-0.01em] text-stone-900",
                        "{job.name}"
                    }
                    if delivers_to_flowflow(&job) {
                        span {
                            class: "shrink-0 text-ios-orange",
                            "aria-label": t(&lang, "hermes-job-flowflow-badge"),
                            IconBell { size: 14 }
                        }
                    }
                }
                p { class: "text-stone-600 text-sm mt-0.5",
                    {schedule_text(&lang, &job.schedule, shift)}
                }
                div { class: "mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs",
                    if paused {
                        span { class: "h-6 px-2 inline-flex items-center rounded-full bg-stone-900/5 font-medium text-stone-500",
                            {t(&lang, "hermes-jobs-paused")}
                        }
                    } else if let Some(when) = next {
                        span { class: "text-stone-500",
                            {t_args(&lang, "hermes-jobs-next", &[("when", &when)])}
                        }
                    }
                    JobStatus { job: job.clone(), lang: lang.clone() }
                }
            }
            button {
                class: "shrink-0 w-11 h-11 flex items-center justify-center rounded-full text-stone-500 active:bg-stone-100",
                "aria-label": t(&lang, "hermes-jobs-actions"),
                onclick: move |e| {
                    e.stop_propagation();
                    tapped_y.set(e.client_coordinates().y);
                    on_menu.call(());
                },
                IconDotsThree { size: 22 }
            }
            if menu_open {
                div {
                    class: "fixed right-5 {kit::MENU_PANEL}",
                    style: "top: min({tapped_y() + 24.0}px, calc(100vh - {MENU_H}px - {EDGE}px - env(safe-area-inset-bottom)));",
                    onclick: move |e| e.stop_propagation(),
                    JobMenuItems { paused, lang: lang.clone(), on_pick }
                }
            }
        }
    }
}
