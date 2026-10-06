use crate::ui::sf_icon::SfIcon;
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
    /// Skills called on purpose, as pills under the bubble.
    #[props(default)]
    skills: Vec<String>,
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
            if !attachments.is_empty() || !skills.is_empty() {
                div { class: "flex flex-wrap justify-end items-center gap-1.5 mt-1",
                    for name in skills.iter() {
                        span {
                            key: "{name}",
                            class: "inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-ios-orange/10 text-ios-orange-dark text-xs font-medium",
                            SfIcon { name: "sparkles", size: 11 }
                            "{name}"
                        }
                    }
                    if !attachments.is_empty() {
                        span { class: "max-w-[80%] text-xs text-stone-400 text-right break-words",
                            {attachments.join(" · ")}
                        }
                    }
                }
            }
        }
    }
}
