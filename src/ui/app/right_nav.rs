use crate::ui::hooks::swipe::use_swipe_right_nav;
use crate::ui::{AppState, View};
use dioxus::prelude::*;

// Right-edge swipe -> open a new chat, from any page. Scoped to the folder we
// are in (context preserved). The left edge stays the burger drawer. No
// rendered element, so it never blocks taps or buttons.
#[component]
pub fn RightNav() -> Element {
    let app: AppState = use_context();

    use_swipe_right_nav(30.0, 70.0, move || {
        let mut app = app;
        let current = (app.view)();
        if matches!(
            current,
            View::Chat {
                conversation_id: None
            }
        ) {
            return;
        }
        app.show_folder_picker.set(false);
        let scope =
            (app.selected_folder_id)().map(crate::domain::ChatScope::Folder);
        app.pending_chat_scope.set(scope);
        app.previous_view.set(Some(current));
        app.view.set(View::Chat {
            conversation_id: None,
        });
    });

    rsx! {
        // Right-edge touch zone (touch-action: none) so the chat swipe is not
        // scroll-stolen; the top bar sits above it so its buttons stay tappable.
        div {
            class: "fixed right-0 top-0 h-full w-8 z-30 lg:hidden",
            style: "touch-action: none;",
        }
    }
}
