use crate::ui::kit;
use dioxus::prelude::*;

#[component]
pub fn DeleteConfirm(
    title: String,
    warning: String,
    cancel_label: String,
    confirm_label: String,
    on_cancel: EventHandler<MouseEvent>,
    on_confirm: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "px-3 py-2.5",
            p { class: "text-sm font-semibold text-stone-900 mb-0.5", "{title}" }
            p { class: "text-xs text-stone-500 mb-3", "{warning}" }
            div { class: "flex gap-2",
                button {
                    class: kit::CONFIRM_BTN_GHOST,
                    onclick: move |e| on_cancel.call(e),
                    "{cancel_label}"
                }
                button {
                    class: kit::CONFIRM_BTN_DANGER,
                    onclick: move |e| on_confirm.call(e),
                    "{confirm_label}"
                }
            }
        }
    }
}
