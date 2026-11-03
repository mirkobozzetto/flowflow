#![cfg(target_os = "macos")]
use flowflow::infrastructure::platform::{device_log, log_to_file};

const MAC_LOG: &str = "Library/Logs/FlowFlow/flowflow.log";

// One test: HOME and the file switch are process-wide.
#[test]
fn the_mac_app_keeps_its_log_in_library_logs_once_switched_on() {
    let home = tempfile::tempdir().expect("tempdir");
    std::env::set_var("HOME", home.path());
    let log = home.path().join(MAC_LOG);

    device_log("[chapters] clip-0: start");
    assert!(!log.exists(), "a test run must not write the app's log");

    log_to_file();
    device_log("[chapters] clip-1: start, 12 words in 2 paragraphs");
    let written = std::fs::read_to_string(&log).expect("log file");
    assert!(
        written.contains("[chapters] clip-1: start, 12 words in 2 paragraphs"),
        "{written}"
    );
}
