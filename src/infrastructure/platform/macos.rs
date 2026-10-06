use std::process::Child;
use std::sync::Mutex;

static PLAYER: Mutex<Option<Child>> = Mutex::new(None);

// Longest side of a photo sent to Hermes, the same as on iOS.
const PHOTO_MAX_SIDE: &str = "1568";
const PHOTO_TYPES: &[&str] =
    &["jpg", "jpeg", "png", "heic", "gif", "webp", "tiff"];

/// Photos chosen in a file dialog, redrawn by sips as (name, JPEG).
pub fn pick_photos() -> Vec<(String, Vec<u8>)> {
    let Some(paths) = rfd::FileDialog::new()
        .add_filter("Images", PHOTO_TYPES)
        .pick_files()
    else {
        return Vec::new();
    };
    paths
        .iter()
        .filter_map(|path| {
            let out = std::env::temp_dir().join(format!(
                "flowflow-{}.jpg",
                uuid::Uuid::new_v4().simple()
            ));
            let reduced = std::process::Command::new("/usr/bin/sips")
                .args(["-Z", PHOTO_MAX_SIDE, "-s", "format", "jpeg"])
                .args(["-s", "formatOptions", "80"])
                .arg(path)
                .arg("--out")
                .arg(&out)
                .output()
                .is_ok_and(|o| o.status.success());
            let bytes = reduced.then(|| std::fs::read(&out).ok()).flatten();
            let _ = std::fs::remove_file(&out);
            let stem = path.file_stem()?.to_string_lossy().to_string();
            Some((format!("{stem}.jpg"), bytes?))
        })
        .collect()
}

pub fn open_calendar_at(
    year: Option<i32>,
    month: Option<i32>,
    day: Option<i32>,
    hour: Option<i32>,
    minute: Option<i32>,
) {
    std::thread::spawn(move || {
        let script = match (year, month, day) {
            (Some(y), Some(m), Some(d)) => Some(format!(
                "set t to current date\n\
                 set day of t to 1\n\
                 set year of t to {y}\n\
                 set month of t to {m}\n\
                 set day of t to {d}\n\
                 set hours of t to {h}\n\
                 set minutes of t to {mi}\n\
                 set seconds of t to 0\n\
                 tell application \"Calendar\"\n\
                 activate\n\
                 switch view to day view\n\
                 view calendar at t\n\
                 end tell",
                h = hour.unwrap_or(9),
                mi = minute.unwrap_or(0),
            )),
            _ => None,
        };
        let ok = script
            .map(|s| {
                std::process::Command::new("/usr/bin/osascript")
                    .arg("-e")
                    .arg(s)
                    .status()
                    .map(|st| st.success())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if !ok {
            let _ = std::process::Command::new("/usr/bin/open")
                .args(["-a", "Calendar"])
                .status();
        }
    });
}

pub fn open_url(url: &str) {
    let _ = std::process::Command::new("/usr/bin/open").arg(url).spawn();
}

pub fn play_audio(path: &str) {
    stop_audio();
    eprintln!("[player] afplay path: {path}");
    if !std::path::Path::new(path).exists() {
        eprintln!("[player] file not found: {path}");
        return;
    }
    match std::process::Command::new("/usr/bin/afplay")
        .arg(path)
        .spawn()
    {
        Ok(child) => {
            *PLAYER.lock().unwrap() = Some(child);
        }
        Err(e) => {
            eprintln!("[player] afplay spawn failed: {e}");
        }
    }
}

pub fn stop_audio() {
    if let Some(mut child) = PLAYER.lock().unwrap().take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn is_playing() -> bool {
    let mut guard = PLAYER.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(None) => true,
            _ => {
                *guard = None;
                false
            }
        }
    } else {
        false
    }
}
