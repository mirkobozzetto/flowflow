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
    pub edge: &'static str, // "left" | "right"
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
    edge: &str,
    edge_px: f64,
    open_at: f64,
    close_at: f64,
) -> String {
    SWIPE_JS
        .replace("__PANEL__", panel_id)
        .replace("__BACKDROP__", backdrop_id)
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
