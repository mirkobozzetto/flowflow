use dioxus::prelude::*;

#[component]
pub fn ChatEmptyState() -> Element {
    rsx! {
        div {
            class: "flex flex-col items-center justify-center px-6 h-full",
            img {
                src: asset!("/assets/flowflow-icon-300.png"),
                width: "150",
                height: "150",
                class: "mb-6 rounded-3xl",
            }
            p { class: "text-stone-900 font-semibold text-base mb-1",
                "Chat avec tes notes"
            }
            p { class: "text-stone-400 text-sm text-center",
                "Pose une question, je cherche dans tes notes."
            }
        }
    }
}
