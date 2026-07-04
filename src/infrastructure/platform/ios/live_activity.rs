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
