use std::ffi::{c_char, CString};

extern "C" {
    fn flowflow_hermes_watch_boot();
    fn flowflow_hermes_watch_run(
        url: *const c_char,
        key: *const c_char,
        label: *const c_char,
    );
}

/// At launch, before iOS hands back the runs it followed while the app slept.
pub fn boot() {
    unsafe { flowflow_hermes_watch_boot() };
}

/// iOS follows the run's event stream while the app sleeps and wakes it at
/// the end; `label` is the JSON the alert is drawn from.
pub fn watch_run(events_url: &str, key: &str, label: &str) {
    let (Ok(url), Ok(key), Ok(label)) = (
        CString::new(events_url),
        CString::new(key),
        CString::new(label),
    ) else {
        return;
    };
    unsafe {
        flowflow_hermes_watch_run(url.as_ptr(), key.as_ptr(), label.as_ptr())
    };
}
