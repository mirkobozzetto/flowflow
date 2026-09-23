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
