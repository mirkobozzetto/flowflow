use super::alert::JobAlert;
use super::card::JobStatus;
use super::menu::JobMenuItems;
use crate::application::hermes_jobs::{
    self, clock_shift, last_run_text, next_run_text, schedule_text, JobAction,
};
use crate::application::i18n::t;
use crate::infrastructure::hermes::HermesError;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::{hermes_problem_text, HermesReply};
use crate::ui::kit;
use crate::ui::{AppState, View};
use chrono::Local;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
fn Fact(label: String, children: Element) -> Element {
    rsx! {
        div { class: "flex justify-between gap-3",
            span { class: "text-stone-500 shrink-0", "{label}" }
            span { class: "text-stone-900 text-right", {children} }
        }
    }
}

/// One task: when it runs, how it last went, and what Hermes answered then,
/// shown as a Hermes reply. "…" in the top bar opens its actions.
#[component]
pub fn HermesJobView() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let job_id = match (app.view)() {
        View::HermesJob { job_id } => job_id,
        _ => String::new(),
    };
    let mut result: Signal<Option<Result<Option<String>, HermesError>>> =
        use_signal(|| None);
    let mut asking: Signal<Option<JobAction>> = use_signal(|| None);
    use_hook({
        let id = job_id.clone();
        move || {
            app.show_job_menu.set(false);
            spawn(async move {
                let database = db();
                if app.hermes_jobs.peek().is_none() {
                    super::reload(app, &database).await;
                }
                result.set(Some(
                    hermes_jobs::latest_result(&database, &id).await,
                ));
            });
        }
    });
    let list = match (app.hermes_jobs)() {
        Some(Ok(list)) => list,
        _ => Vec::new(),
    };
    let shift = clock_shift(&list, &Local);
    let Some(job) = list.into_iter().find(|j| j.id == job_id) else {
        return rsx! {};
    };
    let now = Local::now();
    let paused = job.paused();
    let schedule = schedule_text(&lang, &job.schedule, shift);
    let next = if paused {
        t(&lang, "hermes-jobs-paused")
    } else {
        next_run_text(&lang, &job, &now).unwrap_or_default()
    };
    let failed = job.last_status.as_deref().is_some_and(|s| s != "ok");

    rsx! {
        div { class: "h-full overflow-y-auto px-4 pt-4 safe-pb-40 lg:px-[max(1rem,calc((100%-48rem)/2))]",
            div { class: "bg-warm-white p-4 border border-stone-200 rounded-xl mb-4 space-y-2 text-sm",
                Fact { label: t(&lang, "hermes-job-frequency"), "{schedule}" }
                Fact { label: t(&lang, "hermes-job-next"), "{next}" }
                Fact { label: t(&lang, "hermes-job-last"),
                    span { class: "inline-flex items-center gap-1.5",
                        if let Some(when) = last_run_text(&lang, &job, &now) {
                            "{when} ·"
                        }
                        JobStatus { job: job.clone(), lang: lang.clone() }
                    }
                }
            }
            if failed {
                p { class: "mb-3 text-sm text-ios-red-dark", {t(&lang, "hermes-job-last-failed")} }
            }
            match result() {
                None => rsx! { HermesReply { text: String::new(), steps: Vec::new(), live: true } },
                Some(Err(e)) => rsx! {
                    p { class: "text-sm text-stone-700 leading-relaxed", {hermes_problem_text(&lang, &e)} }
                },
                Some(Ok(None)) => rsx! {
                    p { class: "pt-6 text-sm text-stone-400 text-center", {t(&lang, "hermes-job-no-result")} }
                },
                Some(Ok(Some(text))) => rsx! { HermesReply { text, steps: Vec::new() } },
            }
        }
        div {
            "data-native-menu": "job-more",
            "data-native-context": "{job.id}",
            hidden: !(app.show_job_menu)(),
            div { class: "fixed inset-0 z-40", onclick: move |_| app.show_job_menu.set(false) }
            div { class: "absolute right-4 top-1 {kit::MENU_PANEL}",
                JobMenuItems {
                    paused,
                    lang: lang.clone(),
                    on_pick: move |action| {
                        app.show_job_menu.set(false);
                        asking.set(Some(action));
                    },
                }
            }
        }
        if let Some(action) = asking() {
            JobAlert {
                job_id: job.id.clone(),
                name: job.name.clone(),
                schedule,
                action,
                on_close: move |_| asking.set(None),
            }
        }
    }
}
