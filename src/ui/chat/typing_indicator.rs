use dioxus::prelude::*;

#[component]
pub fn TypingIndicator(tool_status: Option<String>) -> Element {
    rsx! {
        div {
            class: "flex justify-start",
            style: "animation: fadeInUp 0.15s ease-out;",
            div { class: "bg-warm-white border border-ios-orange/10 rounded-2xl rounded-bl-md px-5 py-3.5 shadow-card",
                if let Some(ref status) = tool_status {
                    div { class: "flex items-center gap-2",
                        span {
                            class: "w-1.5 h-1.5 rounded-full bg-ios-orange",
                            style: "animation: pulseSoft 1.2s ease-in-out infinite;",
                        }
                        span { class: "text-xs text-stone-500", "{status}" }
                    }
                } else {
                    div { class: "flex items-center gap-1.5",
                        span {
                            class: "w-1.5 h-1.5 rounded-full bg-ios-orange/60",
                            style: "animation: typingDot 1.2s ease-in-out infinite;",
                        }
                        span {
                            class: "w-1.5 h-1.5 rounded-full bg-ios-orange/60",
                            style: "animation: typingDot 1.2s ease-in-out 0.12s infinite;",
                        }
                        span {
                            class: "w-1.5 h-1.5 rounded-full bg-ios-orange/60",
                            style: "animation: typingDot 1.2s ease-in-out 0.24s infinite;",
                        }
                    }
                }
            }
        }
    }
}
