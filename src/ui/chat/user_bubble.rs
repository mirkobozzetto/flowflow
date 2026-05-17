use dioxus::prelude::*;

#[component]
pub fn UserBubble(text: String) -> Element {
    rsx! {
        div {
            class: "flex justify-end",
            style: "animation: fadeInUp 0.15s ease-out;",
            div {
                class: "bg-ios-orange text-white rounded-2xl rounded-br-md px-4 py-2.5 max-w-[80%] text-sm leading-relaxed break-words",
                "{text}"
            }
        }
    }
}
