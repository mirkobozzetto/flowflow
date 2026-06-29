use crate::ui::AppState;
use dioxus::prelude::*;

// Repeatable history navigation (the same back/forward stack driven by Cmd+Left
// / Cmd+Right on desktop). Each call pops one view off the history.
pub fn nav_back(mut app: AppState) {
    // Prefer the history stack; fall back to previous_view so back works from
    // any page even if the stack was not populated.
    let popped = app.view_history.write().pop();
    let target = popped.or_else(|| (app.previous_view)());
    if let Some(target) = target {
        app.view_future.write().push(app.view.peek().clone());
        app.history_nav.set(true);
        // Reset transient nav/overlay state so the previous and next views never
        // render layered on top of each other.
        app.sliding_out.set(false);
        app.previous_view.set(None);
        app.show_folder_picker.set(false);
        app.show_note_menu.set(false);
        app.show_chat_menu.set(false);
        app.show_thread_menu.set(false);
        app.view.set(target);
    }
}
