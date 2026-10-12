//! A Hermes task followed in the Dynamic Island. Hermes announces no start,
//! so the server starts the Live Activity at the task's next run and ends it
//! with the result; the phone hands over what only it knows: when the
//! FlowFlow-delivered tasks run next, and ActivityKit's tokens.

use crate::application::hermes_delivery::delivers_to_flowflow;
use crate::application::push;
use crate::infrastructure::backend::{BackendClient, ScheduledJob};
use crate::infrastructure::hermes::HermesJob;
use crate::infrastructure::persistence::Database;
use std::sync::Mutex;

// ActivityKit hands it out once per launch; the jobs list needs it later.
static START_TOKEN: Mutex<Option<String>> = Mutex::new(None);

/// Enabled tasks delivered to FlowFlow, at their next run.
pub fn scheduled(jobs: &[HermesJob]) -> Vec<ScheduledJob> {
    jobs.iter()
        .filter(|j| j.enabled && delivers_to_flowflow(j))
        .filter_map(|j| {
            let at =
                chrono::DateTime::parse_from_rfc3339(j.next_run_at.as_deref()?)
                    .ok()?;
            Some(ScheduledJob {
                id: j.id.clone(),
                name: j.name.clone(),
                next_run_at: at.timestamp(),
            })
        })
        .collect()
}

fn start_token() -> Option<String> {
    START_TOKEN
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Sends the server the next runs. Silent without the token (iOS before
/// 17.2, or not handed out yet) or with notifications off: the alert alone
/// then tells the result.
pub async fn sync(db: &Database, jobs: &[HermesJob]) {
    let Some(token) = start_token() else { return };
    if !push::is_enabled(db) {
        return;
    }
    let Some(backend) = BackendClient::from_db(db) else {
        return;
    };
    let jobs = scheduled(jobs);
    match backend.push_schedules(db, &token, &jobs).await {
        Ok(()) => eprintln!("[live] {} task(s) scheduled", jobs.len()),
        Err(e) => eprintln!("[live] schedules not sent: {e}"),
    }
}

#[cfg(target_os = "ios")]
async fn report(db: &Database, job_id: &str, token: &str) {
    let Some(backend) = BackendClient::from_db(db) else {
        return;
    };
    match backend.report_activity(db, job_id, token).await {
        Ok(()) => eprintln!("[live] activity token sent"),
        Err(e) => eprintln!("[live] activity token not sent: {e}"),
    }
}

/// Runs for the whole launch: a push may start an activity while the app
/// sleeps, and iOS wakes it to hand over the token that ends it.
#[cfg(target_os = "ios")]
pub async fn follow(db: std::sync::Arc<Database>) {
    use crate::infrastructure::platform::ios::live_activity::{
        observe_hermes, HermesEvent,
    };
    let Some(mut events) = observe_hermes() else {
        return;
    };
    while let Some(event) = events.recv().await {
        match event {
            HermesEvent::StartToken(token) => {
                eprintln!("[live] push-to-start token received");
                *START_TOKEN.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(token);
                if push::is_enabled(&db) {
                    if let Ok(jobs) =
                        crate::application::hermes_jobs::list(&db).await
                    {
                        sync(&db, &jobs).await;
                    }
                }
            }
            HermesEvent::ActivityToken { job_id, token } => {
                report(&db, &job_id, &token).await
            }
        }
    }
}
