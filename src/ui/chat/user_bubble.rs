use dioxus::prelude::*;

#[component]
pub fn UserBubble(
    text: String,
    /// Ink instead of orange: the question is going to Hermes.
    #[props(default)]
    ink: bool,
) -> Element {
    let tone = if ink { "bg-stone-900" } else { "bg-ios-orange" };
    rsx! {
        div {
            class: "flex justify-end",
            style: "animation: fadeInUp 0.15s ease-out;",
            div {
                class: "{tone} text-white rounded-2xl rounded-br-md px-4 py-2.5 max-w-[80%] text-sm leading-relaxed break-words",
                "{text}"
            }
        }
    }
}
