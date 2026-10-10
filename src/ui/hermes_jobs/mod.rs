//! Hermes' scheduled tasks: the list, one task with its last result, the
//! new-task sheet, and the Hermes alert every action goes through.

mod alert;
mod card;
mod entries;
mod form;
mod job_view;
mod list;
mod menu;

pub use entries::{
    facts_link, known_count, new_task, HermesMoreMenu, JobsEntries,
};
pub use job_view::HermesJobView;
pub use list::HermesJobsView;

use crate::application::hermes_jobs;
use crate::infrastructure::persistence::Database;
use crate::ui::{AppState, View};
use dioxus::prelude::*;

/// Opens the tasks list from wherever Hermes is, which the back button
/// returns to.
pub fn open(mut app: AppState) {
    app.previous_view.set(Some((app.view)()));
    app.view.set(View::HermesJobs);
}

/// Reads the tasks again; the list on screen stays until the new one lands.
pub async fn reload(mut app: AppState, db: &Database) {
    app.hermes_jobs.set(Some(hermes_jobs::list(db).await));
}
