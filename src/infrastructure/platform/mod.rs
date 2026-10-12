/// Discreet feedback for the composer and menus; desktop intentionally stays silent.
/// Kinds: "selection", "light", "medium", "soft". `haptic_prepare` warms the same
/// generator on pointerdown so the tap itself fires without latency.
pub fn haptic(kind: &str) {
    #[cfg(target_os = "ios")]
    ios_haptics::fire(kind);
    #[cfg(not(target_os = "ios"))]
    let _ = kind;
}

pub fn haptic_prepare(kind: &str) {
    #[cfg(target_os = "ios")]
    ios_haptics::prepare(kind);
    #[cfg(not(target_os = "ios"))]
    let _ = kind;
}

/// Where `device_log` keeps its lines on iOS, read back from the Mac with
/// `devicectl device copy from` once the phone is unlocked.
#[cfg(target_os = "ios")]
const DEVICE_LOG: &str = "Library/Caches/flowflow.log";
// ponytail: one file, started over past this size.
#[cfg(target_os = "ios")]
const DEVICE_LOG_MAX: u64 = 512 * 1024;

/// A line in the app's log. On iOS it also goes to the system log and to
/// DEVICE_LOG: the console drops off as soon as the phone locks.
pub fn device_log(msg: &str) {
    eprintln!("{msg}");
    #[cfg(target_os = "ios")]
    {
        use std::io::Write;
        if let Ok(home) = std::env::var("HOME") {
            let path = std::path::Path::new(&home).join(DEVICE_LOG);
            let full = std::fs::metadata(&path)
                .is_ok_and(|m| m.len() > DEVICE_LOG_MAX);
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(!full)
                .write(true)
                .truncate(full)
                .open(&path)
            {
                let at = chrono::Utc::now().format("%H:%M:%S%.3f");
                let _ = writeln!(file, "{at} {msg}");
            }
        }
    }
    #[cfg(target_os = "ios")]
    {
        use std::ffi::CString;
        extern "C" {
            fn syslog(priority: i32, format: *const std::ffi::c_char, ...);
        }
        if let Ok(cmsg) = CString::new(format!("[FlowFlow] {msg}")) {
            unsafe {
                syslog(
                    4,
                    b"%s\0".as_ptr() as *const std::ffi::c_char,
                    cmsg.as_ptr(),
                );
            }
        }
    }
}

/// Stops the screen from auto-locking; main thread only, a no-op off iOS.
pub fn keep_screen_awake(on: bool) {
    #[cfg(target_os = "ios")]
    ios::keep_screen_awake(on);
    #[cfg(not(target_os = "ios"))]
    let _ = on;
}

#[cfg(target_os = "ios")]
mod ios_haptics {
    use objc2::rc::Retained;
    use objc2::MainThreadMarker;
    use objc2_ui_kit::{
        UIFeedbackGenerator, UIImpactFeedbackGenerator, UIImpactFeedbackStyle,
        UISelectionFeedbackGenerator,
    };
    use std::cell::RefCell;
    use std::collections::HashMap;

    // One generator per kind, kept alive so `prepare` benefits the next tap.
    // Generators are main-thread only; a call off the main thread is a no-op.
    thread_local! {
        static GENERATORS: RefCell<HashMap<&'static str, Retained<UIFeedbackGenerator>>> =
            RefCell::new(HashMap::new());
    }

    fn generator(kind: &str) -> Option<Retained<UIFeedbackGenerator>> {
        let mtm = MainThreadMarker::new()?;
        let (key, style) = match kind {
            "selection" => ("selection", None),
            "light" => ("light", Some(UIImpactFeedbackStyle::Light)),
            "medium" => ("medium", Some(UIImpactFeedbackStyle::Medium)),
            "soft" => ("soft", Some(UIImpactFeedbackStyle::Soft)),
            _ => return None,
        };
        GENERATORS.with(|generators| {
            let mut generators = generators.borrow_mut();
            let generator =
                generators.entry(key).or_insert_with(|| match style {
                    None => Retained::into_super(
                        UISelectionFeedbackGenerator::new(mtm),
                    ),
                    #[allow(deprecated)]
                    Some(style) => Retained::into_super(
                        UIImpactFeedbackGenerator::initWithStyle(
                            mtm.alloc(),
                            style,
                        ),
                    ),
                });
            Some(generator.clone())
        })
    }

    pub fn prepare(kind: &str) {
        if let Some(generator) = generator(kind) {
            generator.prepare();
        }
    }

    pub fn fire(kind: &str) {
        let Some(generator) = generator(kind) else {
            return;
        };
        if kind == "selection" {
            if let Ok(selection) =
                Retained::downcast::<UISelectionFeedbackGenerator>(generator)
            {
                selection.selectionChanged();
            }
        } else if let Ok(impact) =
            Retained::downcast::<UIImpactFeedbackGenerator>(generator)
        {
            impact.impactOccurred();
        }
    }
}

#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod aac;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod parsers;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod pdf;

#[cfg(target_os = "ios")]
pub fn detect_system_language() -> String {
    ios::detect_system_language()
}

#[cfg(not(target_os = "ios"))]
pub fn detect_system_language() -> String {
    "en".to_string()
}

#[cfg(target_os = "ios")]
pub fn open_url(url: &str) {
    ios::open_url(url);
}

#[cfg(target_os = "macos")]
pub fn open_url(url: &str) {
    macos::open_url(url);
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
pub fn open_url(_url: &str) {}

/// Photos chosen for a Hermes question, reduced to JPEGs: (name, bytes).
pub async fn pick_photos() -> Vec<(String, Vec<u8>)> {
    #[cfg(target_os = "ios")]
    return ios::pick_photos().await;
    #[cfg(target_os = "macos")]
    return macos::pick_photos();
    #[allow(unreachable_code)]
    Vec::new()
}

/// A photo taken now, on a device with a camera.
pub async fn take_photo() -> Option<(String, Vec<u8>)> {
    #[cfg(target_os = "ios")]
    return ios::take_photo().await;
    #[allow(unreachable_code)]
    None
}

pub const HAS_CAMERA: bool = cfg!(target_os = "ios");
