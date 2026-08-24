use crate::infrastructure::persistence::Database;
use crate::ui::{AppState, View};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn FloatingActionButton() -> Element {
    let mut app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let mut clicked = use_signal(|| false);
    let mut pressing = use_signal(|| false);

    // No compose button on a theme this user cannot write to. The notes list
    // says why, right above; an inert button with no reason would be worse.
    let read_only = use_memo(move || {
        let _v = (app.folders_version)();
        matches!(
            (app.selected_folder_id)().map(|fid| {
                crate::application::space::folder_right(&db(), &fid)
            }),
            Some(crate::application::space::FolderRight::SpaceReadOnly)
        )
    });
    if read_only() {
        return rsx! {};
    }

    rsx! {
        div { class: "fixed safe-bottom-6 right-4 z-20",
            button {
                class: "fab-btn",
                class: if pressing() { "fab-pressing" },
                class: if clicked() { "fab-clicked" },
                onpointerdown: move |_| pressing.set(true),
                onpointerup: move |_| pressing.set(false),
                onpointerleave: move |_| pressing.set(false),
                onclick: move |_| {
                    if clicked() {
                        return;
                    }
                    clicked.set(true);
                    spawn(async move {
                        futures_timer::Delay::new(
                            std::time::Duration::from_millis(150),
                        )
                        .await;
                        app.show_folder_picker.set(false);
                        app.view.set(View::NoteDetail { note_id: String::new() });
                        clicked.set(false);
                    });
                },
                svg {
                    width: "52",
                    height: "52",
                    view_box: "0 0 100 100",
                    line {
                        class: "fab-plus-h",
                        x1: "30",
                        y1: "50",
                        x2: "70",
                        y2: "50",
                    }
                    line {
                        class: "fab-plus-v",
                        x1: "50",
                        y1: "30",
                        x2: "50",
                        y2: "70",
                    }
                }
            }
        }
    }
}
