use dioxus::prelude::*;

/// #177: on iOS 26+, lay native Liquid Glass buttons over the web anchors
/// (`data-glass`: burger, chat pill, new-note button) and keep them on them
/// (place, card drag, card glide). Elsewhere, and whenever a native button is
/// unavailable, the web anchor is the button.
pub fn use_glass_burger() {
    use_future(|| async {
        #[cfg(target_os = "ios")]
        {
            use crate::infrastructure::platform::ios::glass_burger as native;
            const GLASS_JS: &str = include_str!("glass_burger.js");

            // The web view joins the key window after the first frames.
            let mut installed = false;
            for _ in 0..30 {
                installed = native::install();
                if installed {
                    break;
                }
                futures_timer::Delay::new(std::time::Duration::from_millis(
                    100,
                ))
                .await;
            }
            if !installed {
                return;
            }
            let mut eval = dioxus::document::eval(GLASS_JS);
            while let Ok(msg) = eval.recv::<String>().await {
                let mut parts = msg.split(' ');
                let kind = parts.next();
                let id = if kind == Some("place") {
                    parts.next()
                } else {
                    None
                };
                let nums: Vec<f64> =
                    parts.filter_map(|s| s.parse().ok()).collect();
                match (kind, nums.as_slice()) {
                    (Some("place"), &[x, y, w, h, travel, visible, dot]) => {
                        native::place(
                            id.unwrap_or_default(),
                            x,
                            y,
                            w,
                            h,
                            travel,
                            visible > 0.5,
                            dot > 0.5,
                        )
                    }
                    (Some("drag"), &[p]) => native::drag(p),
                    (Some("settle"), &[p]) => native::settle(p),
                    _ => {}
                }
            }
        }
    });
}
