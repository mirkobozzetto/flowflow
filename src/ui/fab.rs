use crate::ui::{AppState, View};
use dioxus::prelude::*;

#[component]
pub fn FloatingActionButton() -> Element {
    let mut app: AppState = use_context();
    let mut clicked = use_signal(|| false);
    let mut pressing = use_signal(|| false);

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
