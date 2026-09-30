use crate::infrastructure::transcription::gpu_gate;
use std::ffi::{c_char, CString};
use std::sync::Once;

extern "C" {
    fn flowflow_register_continued_transcription(on_run: extern "C" fn(bool));
    fn flowflow_begin_continued_transcription(
        title: *const c_char,
        subtitle: *const c_char,
    ) -> bool;
    fn flowflow_continued_transcription_progress(
        title: *const c_char,
        subtitle: *const c_char,
        done: i64,
        total: i64,
    );
    fn flowflow_end_continued_transcription(success: bool);
}

static REGISTER: Once = Once::new();

extern "C" fn on_run(running: bool) {
    gpu_gate::allow_cpu(running);
}

fn c_string(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// Also the link anchor that keeps the Swift file in the binary.
pub fn register() {
    REGISTER.call_once(|| unsafe {
        flowflow_register_continued_transcription(on_run);
    });
}

/// `false` below iOS 26 or when iOS refuses: the job then pauses in the
/// background as before.
pub fn begin(title: &str, subtitle: &str) -> bool {
    let (title, subtitle) = (c_string(title), c_string(subtitle));
    unsafe {
        flowflow_begin_continued_transcription(
            title.as_ptr(),
            subtitle.as_ptr(),
        )
    }
}

pub fn progress(title: &str, subtitle: &str, done_ms: u32, total_ms: u32) {
    let (title, subtitle) = (c_string(title), c_string(subtitle));
    unsafe {
        flowflow_continued_transcription_progress(
            title.as_ptr(),
            subtitle.as_ptr(),
            i64::from(done_ms),
            i64::from(total_ms),
        );
    }
}

pub fn end(success: bool) {
    gpu_gate::allow_cpu(false);
    unsafe {
        flowflow_end_continued_transcription(success);
    }
}
