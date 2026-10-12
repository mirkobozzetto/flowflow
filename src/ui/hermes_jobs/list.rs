use super::alert::{CreatedAlert, JobAlert};
use super::card::JobCard;
use super::form::NewJobSheet;
use crate::application::hermes_jobs::{
    clock_shift, next_run_text, schedule_text, JobAction,
};
use crate::application::i18n::{t, t_args};
use crate::infrastructure::hermes::HermesJob;
use crate::infrastructure::persistence::Database;
use crate::ui::chat::hermes_problem_text;
use crate::ui::icons::HermesAgentIcon;
use crate::ui::kit;
use crate::ui::state::{SettingsSection, View};
use crate::ui::AppState;
use chrono::Local;
use dioxus::prelude::*;
use std::sync::Arc;

/// Hermes' scheduled tasks, read again on each opening.
#[component]
pub fn HermesJobsView() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    // The task whose "…" menu is open, the action waiting on its alert, and
    // the task just created.
    let mut menu_for: Signal<Option<String>> = use_signal(|| None);
    let mut asking: Signal<Option<(HermesJob, JobAction)>> =
        use_signal(|| None);
    let mut created: Signal<Option<HermesJob>> = use_signal(|| None);
    use_hook(move || {
        spawn(async move { super::reload(app, &db()).await });
    });
    let jobs = (app.hermes_jobs)();
    let shift = match &jobs {
        Some(Ok(list)) => clock_shift(list, &Local),
        _ => 0,
    };

    rsx! {
        div { class: "h-full overflow-y-auto px-4 pt-4 safe-pb-40 lg:px-[max(1rem,calc((100%-48rem)/2))]",
            match jobs {
                None => rsx! {},
                Some(Err(e)) => rsx! {
                    div { class: "rounded-xl border border-stone-200 bg-warm-white p-4 mb-3 space-y-3",
                        p { class: "text-sm text-stone-700 leading-relaxed", {hermes_problem_text(&lang, &e)} }
                        button {
                            class: kit::PILL_PRIMARY,
                            onclick: move |_| {
                                app.previous_view.set(Some((app.view)()));
                                app.view.set(View::SettingsSection(SettingsSection::Connections));
                            },
                            {t(&lang, "hermes-open-settings")}
                        }
                    }
                },
                Some(Ok(list)) if list.is_empty() => rsx! {
                    div { class: "flex flex-col items-center text-center pt-24 px-6",
                        HermesAgentIcon { size: 64 }
                        p { class: "mt-4 text-stone-900 font-semibold text-base", {t(&lang, "hermes-jobs-empty-title")} }
                        p { class: "mt-1 text-stone-400 text-sm", {t(&lang, "hermes-jobs-empty-hint")} }
                        button {
                            class: "pressable mt-5 h-11 px-5 rounded-full bg-ios-orange text-white text-[15px] font-semibold",
                            onclick: move |_| app.hermes_job_form.set(true),
                            {t(&lang, "hermes-jobs-new")}
                        }
                    }
                },
                Some(Ok(list)) => rsx! {
                    for job in list {
                        JobCard {
                            key: "{job.id}",
                            job: job.clone(),
                            shift,
                            menu_open: menu_for().as_deref() == Some(job.id.as_str()),
                            on_open: {
                                let id = job.id.clone();
                                move |_| app.view.set(View::HermesJob { job_id: id.clone() })
                            },
                            on_menu: {
                                let id = job.id.clone();
                                move |_| menu_for.set(Some(id.clone()))
                            },
                            on_pick: {
                                let job = job.clone();
                                move |action| {
                                    menu_for.set(None);
                                    asking.set(Some((job.clone(), action)));
                                }
                            },
                        }
                    }
                },
            }
        }
        if menu_for().is_some() {
            div { class: "fixed inset-0 z-40", onclick: move |_| menu_for.set(None) }
        }
        if let Some((job, action)) = asking() {
            JobAlert {
                key: "{job.id}",
                job_id: job.id.clone(),
                name: job.name.clone(),
                schedule: schedule_text(&lang, &job.schedule, shift),
                action,
                on_close: move |_| asking.set(None),
            }
        }
        if (app.hermes_job_form)() {
            NewJobSheet {
                on_close: move |_| app.hermes_job_form.set(false),
                on_created: move |job: HermesJob| {
                    app.hermes_job_form.set(false);
                    created.set(Some(job));
                },
            }
        }
        if let Some(job) = created() {
            CreatedAlert {
                name: job.name.clone(),
                lines: {
                    let own = clock_shift(std::slice::from_ref(&job), &Local);
                    let mut lines = vec![schedule_text(&lang, &job.schedule, own)];
                    if let Some(when) = next_run_text(&lang, &job, &Local::now()) {
                        lines.push(t_args(&lang, "hermes-jobs-next", &[("when", &when)]));
                    }
                    lines
                },
                on_close: move |_| created.set(None),
            }
        }
    }
}
