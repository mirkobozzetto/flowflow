use dioxus::prelude::*;

// Reusable edge-swipe drawer hook. Webview only (it drives the DOM via
// document::eval); on dioxus-native there is no DOM, so it is a no-op there.
//
// The caller renders three things: a panel element (`panel_id`) positioned
// off-screen via a Tailwind `translate` class, a backdrop element
// (`backdrop_id`), and a thin edge element with `touch-action: none` over the
// edge zone (so the open swipe is not stolen by native scroll). The panel keeps
// a `data-open` attribute synced to `open`. The hook injects a pointer
// controller that follows the finger and, on release, sends the committed state
// back here to flip `open`.

#[derive(Clone, Copy)]
pub struct DrawerSwipe {
    pub open: Signal<bool>,
    pub panel_id: &'static str,
    pub backdrop_id: &'static str,
    pub card_id: &'static str, // "" = drawer over the content, else card mode
    pub edge: &'static str,    // "left" | "right"
    pub edge_px: f64,
    pub open_at: f64,
    pub close_at: f64,
}

const SWIPE_JS: &str = include_str!("swipe.js");

// Fill the controller's config placeholders. Kept separate from the hook (which
// needs a Dioxus runtime) so it is unit-testable: a leftover `__PLACEHOLDER__`
// or a `NaN` in the output means the embedded JS broke its config contract.
pub fn build_script(
    panel_id: &str,
    backdrop_id: &str,
    card_id: &str,
    edge: &str,
    edge_px: f64,
    open_at: f64,
    close_at: f64,
) -> String {
    SWIPE_JS
        .replace("__PANEL__", panel_id)
        .replace("__BACKDROP__", backdrop_id)
        .replace("__CARD__", card_id)
        .replace("__EDGE__", edge)
        .replace("__EDGE_PX__", &edge_px.to_string())
        .replace("__OPEN_AT__", &open_at.to_string())
        .replace("__CLOSE_AT__", &close_at.to_string())
}

pub fn use_swipe_drawer(cfg: DrawerSwipe) {
    let mut open = cfg.open;
    use_future(move || async move {
        let script = build_script(
            cfg.panel_id,
            cfg.backdrop_id,
            cfg.card_id,
            cfg.edge,
            cfg.edge_px,
            cfg.open_at,
            cfg.close_at,
        );
        let mut eval = dioxus::document::eval(&script);
        while let Ok(msg) = eval.recv::<String>().await {
            match msg.as_str() {
                "open" => open.set(true),
                "closed" => open.set(false),
                _ => {}
            }
        }
    });
}

const SWIPE_SHEET_JS: &str = include_str!("swipe_sheet.js");

pub fn build_sheet_script(
    sheet_id: &str,
    backdrop_id: &str,
    grab_px: f64,
    dismiss_at: f64,
) -> String {
    SWIPE_SHEET_JS
        .replace("__SHEET__", sheet_id)
        .replace("__BACKDROP__", backdrop_id)
        .replace("__GRAB_PX__", &grab_px.to_string())
        .replace("__DISMISS_AT__", &dismiss_at.to_string())
}

// Bottom-sheet drag-to-dismiss: a downward drag from the grabber strip follows
// the finger and, past the threshold (or on a flick), animates out and runs
// `on_close` so the caller unmounts the sheet. The sheet remounts each open, so
// the controller tears down its prior listeners on re-eval.
pub fn use_sheet_dismiss(
    sheet_id: &'static str,
    backdrop_id: &'static str,
    grab_px: f64,
    dismiss_at: f64,
    on_close: impl FnMut() + 'static,
) {
    let mut on_close = Some(on_close);
    use_future(move || {
        let taken = on_close.take();
        async move {
            let Some(mut on_close) = taken else {
                return;
            };
            let script =
                build_sheet_script(sheet_id, backdrop_id, grab_px, dismiss_at);
            let mut eval = dioxus::document::eval(&script);
            while let Ok(msg) = eval.recv::<String>().await {
                if msg == "closed" {
                    on_close();
                }
            }
        }
    });
}
