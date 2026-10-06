use dioxus::prelude::*;

#[component]
pub fn UserBubble(
    text: String,
    /// Ink instead of orange: the question is going to Hermes.
    #[props(default)]
    ink: bool,
    /// Names of what went with the question, under the bubble.
    #[props(default)]
    attachments: Vec<String>,
) -> Element {
    let tone = if ink { "bg-stone-900" } else { "bg-ios-orange" };
    let text = if text.trim().is_empty() {
        "…".to_string()
    } else {
        text
    };
    rsx! {
        div { style: "animation: fadeInUp 0.15s ease-out;",
            div { class: "flex justify-end",
                div {
                    class: "{tone} text-white rounded-2xl rounded-br-md px-4 py-2.5 max-w-[80%] text-sm leading-relaxed break-words",
                    "{text}"
                }
            }
            if !attachments.is_empty() {
                div { class: "flex justify-end mt-1",
                    span { class: "max-w-[80%] text-xs text-stone-400 text-right break-words",
                        {attachments.join(" · ")}
                    }
                }
            }
        }
    }
}
