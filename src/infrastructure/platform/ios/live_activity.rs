extern "C" {
    fn flowflow_start_live_activity(started_at_unix: i64, is_paused: bool);
    fn flowflow_update_live_activity(started_at_unix: i64, is_paused: bool);
    fn flowflow_end_live_activity();
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
