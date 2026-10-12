//! Hermes' scheduled tasks: the list, one task with its last result, the
//! new-task sheet, and the Hermes alert every action goes through.

mod alert;
mod card;
mod entries;
mod flowflow;
mod form;
mod job_view;
mod list;
mod menu;

pub use entries::{
    facts_link, known_count, new_task, HermesMoreMenu, JobsEntries,
};
pub use job_view::HermesJobView;
pub use list::HermesJobsView;

use crate::application::{hermes_jobs, hermes_live};
use crate::infrastructure::hermes::HermesJob;
use crate::infrastructure::persistence::Database;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

/// Opens the tasks list from wherever Hermes is, which the back button
/// returns to.
pub fn open(mut app: AppState) {
    app.previous_view.set(Some((app.view)()));
    app.view.set(View::HermesJobs);
}

/// A tapped task notification: the task it names, or the list. Read again
/// first, the result that just arrived is newer than the list in memory.
pub async fn open_linked(mut app: AppState, db: &Arc<Database>, link: &str) {
    reload(app, db).await;
    let job_id = match &*app.hermes_jobs.peek() {
        Some(Ok(list)) => {
            hermes_jobs::linked_job(link, list).map(|j| j.id.clone())
        }
        _ => None,
    };
    crate::infrastructure::platform::device_log(&format!(
        "[hermes] notification opens task {job_id:?}"
    ));
    let here = app.view.peek().clone();
    if !matches!(here, View::HermesJobs | View::HermesJob { .. }) {
        app.previous_view.set(Some(here));
    }
    app.sidebar_open.set(false);
    app.view.set(match job_id {
        Some(job_id) => View::HermesJob { job_id },
        None => View::HermesJobs,
    });
}

/// Reads the tasks again; the list on screen stays until the new one lands.
/// Their next runs go to the server, which starts their Live Activities.
pub async fn reload(mut app: AppState, db: &Arc<Database>) {
    let jobs = hermes_jobs::list(db).await;
    if let Ok(list) = &jobs {
        let (db, list) = (db.clone(), list.clone());
        spawn(async move { hermes_live::sync(&db, &list).await });
    }
    app.hermes_jobs.set(Some(jobs));
}

// Hermes' API restarts right after FlowFlow is first set up: the job Hermes
// answered replaces its old self instead of a reload that would fail.
fn replace(mut app: AppState, job: HermesJob) {
    if let Some(Ok(list)) = app.hermes_jobs.write().as_mut() {
        if let Some(slot) = list.iter_mut().find(|j| j.id == job.id) {
            *slot = job;
        }
    }
}

/// A task Hermes deleted leaves the list on screen at once.
fn forget(mut app: AppState, job_id: &str) {
    if let Some(Ok(list)) = app.hermes_jobs.write().as_mut() {
        list.retain(|j| j.id != job_id);
    }
}
