use std::ffi::{c_char, CStr};
use std::sync::OnceLock;
use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver, UnboundedSender,
};

extern "C" {
    fn flowflow_start_live_activity(started_at_unix: i64, is_paused: bool);
    fn flowflow_update_live_activity(started_at_unix: i64, is_paused: bool);
    fn flowflow_end_live_activity();
    fn flowflow_cleanup_live_activities();
    fn flowflow_register_record_intent();
}

/// No-op whose only job is to pull the RecordIntent object file into the
/// link, so the App Intents type exists in the app binary at runtime.
pub fn register_record_intent() {
    unsafe {
        flowflow_register_record_intent();
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn start() {
    unsafe {
        flowflow_start_live_activity(now_unix(), false);
    }
}

pub fn update(is_paused: bool) {
    unsafe {
        flowflow_update_live_activity(now_unix(), is_paused);
    }
}

pub fn end() {
    unsafe {
        flowflow_end_live_activity();
    }
}

/// Boot sweep: end every activity left behind by a killed process (the
/// current one, if any, is spared on the Swift side).
pub fn cleanup_orphans() {
    unsafe {
        flowflow_cleanup_live_activities();
    }
}

extern "C" {
    fn flowflow_hermes_activities_observe(
        start: extern "C" fn(*const c_char),
        token: extern "C" fn(*const c_char, *const c_char),
    );
}

pub enum HermesEvent {
    StartToken(String),
    ActivityToken { job_id: String, token: String },
}

static HERMES_EVENTS: OnceLock<UnboundedSender<HermesEvent>> = OnceLock::new();

fn text(ptr: *const c_char) -> String {
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

extern "C" fn on_start_token(token: *const c_char) {
    if let Some(tx) = HERMES_EVENTS.get() {
        let _ = tx.send(HermesEvent::StartToken(text(token)));
    }
}

extern "C" fn on_activity_token(job_id: *const c_char, token: *const c_char) {
    if let Some(tx) = HERMES_EVENTS.get() {
        let _ = tx.send(HermesEvent::ActivityToken {
            job_id: text(job_id),
            token: text(token),
        });
    }
}

/// ActivityKit's tokens for Hermes task activities, as they come. None
/// when already observed: one launch, one observer.
pub fn observe_hermes() -> Option<UnboundedReceiver<HermesEvent>> {
    let (tx, rx) = unbounded_channel();
    HERMES_EVENTS.set(tx).ok()?;
    unsafe {
        flowflow_hermes_activities_observe(on_start_token, on_activity_token)
    };
    Some(rx)
}
