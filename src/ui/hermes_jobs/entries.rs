//! The ways into the tasks from wherever Hermes is: the chat's "…", its "+",
//! the drawer and the line under Hermes' face all lead to the same list.

use crate::application::i18n::{t, t_args};
use crate::ui::icons::IconPlus;
use crate::ui::kit;
use crate::ui::sf_icon::SfIcon;
use crate::ui::AppState;
use dioxus::prelude::*;

/// Opens the list with the new-task sheet up.
pub fn new_task(mut app: AppState) {
    super::open(app);
    app.hermes_job_form.set(true);
}

/// How many tasks Hermes has, once read.
pub fn known_count(app: AppState) -> Option<usize> {
    match (app.hermes_jobs)() {
        Some(Ok(list)) => Some(list.len()),
        _ => None,
    }
}

/// "4 tâches", "1 tâche", "Aucune tâche".
pub fn count_label(lang: &str, n: usize) -> String {
    match n {
        0 => t(lang, "hermes-jobs-none"),
        1 => t(lang, "hermes-jobs-one"),
        n => t_args(lang, "hermes-jobs-count", &[("count", &n.to_string())]),
    }
}

/// Under Hermes' face: the tasks there are, or the way to the first one.
pub fn facts_link(lang: &str, n: usize) -> String {
    match n {
        0 => t(lang, "hermes-jobs-plan"),
        1 => t(lang, "hermes-facts-job-one"),
        n => t_args(lang, "hermes-facts-jobs", &[("count", &n.to_string())]),
    }
}

/// "Tâches planifiées" and "Nouvelle tâche". The same buttons feed a native
/// iOS menu (`data-native-*`) and draw the web one with `class`.
#[component]
pub fn JobsEntries(class: &'static str, on_pick: EventHandler<()>) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let count = known_count(app).map(|n| count_label(&lang, n));
    rsx! {
        button {
            class,
            "data-native-action": "jobs",
            "data-native-title": t(&lang, "hermes-jobs-title"),
            "data-native-symbol": "clock",
            "data-native-subtitle": count.clone(),
            onclick: move |_| {
                on_pick.call(());
                super::open(app);
            },
            span { class: "w-7 shrink-0 flex justify-center text-stone-500", SfIcon { name: "clock", size: 20 } }
            span { class: "flex-1 flex flex-col gap-0.5 min-w-0 text-left",
                span { class: "text-sm text-stone-800", {t(&lang, "hermes-jobs-title")} }
                if let Some(c) = &count {
                    span { class: "text-xs text-stone-400", "{c}" }
                }
            }
        }
        button {
            class,
            "data-native-action": "new-job",
            "data-native-title": t(&lang, "hermes-jobs-new"),
            "data-native-symbol": "plus",
            onclick: move |_| {
                on_pick.call(());
                new_task(app);
            },
            span { class: "w-7 shrink-0 flex justify-center text-stone-500", IconPlus { size: 18 } }
            span { class: "flex-1 text-left text-sm text-stone-800", {t(&lang, "hermes-jobs-new")} }
        }
    }
}

/// The Hermes chat's "…": a native glass menu on iOS 26, a web one elsewhere.
#[component]
pub fn HermesMoreMenu() -> Element {
    let mut app: AppState = use_context();
    rsx! {
        div {
            "data-native-menu": "hermes-more",
            "data-native-context": "hermes",
            hidden: !(app.show_hermes_menu)(),
            div { class: "fixed inset-0 z-40", onclick: move |_| app.show_hermes_menu.set(false) }
            div { class: "absolute right-4 top-1 {kit::MENU_PANEL}",
                JobsEntries {
                    class: kit::MENU_ITEM,
                    on_pick: move |_| app.show_hermes_menu.set(false),
                }
            }
        }
    }
}
