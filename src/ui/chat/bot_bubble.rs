use crate::ui::chat::actions::md_to_html;
use crate::ui::chat::models::ChatSource;
use crate::ui::chat::sources_accordion::SourcesAccordion;
use dioxus::prelude::*;

#[component]
pub fn BotBubble(
    text: String,
    sources: Vec<ChatSource>,
    conversation_id: Option<String>,
) -> Element {
    rsx! {
        div {
            class: "flex justify-start",
            style: "animation: fadeInUp 0.15s ease-out;",
            div { class: "bg-warm-white border border-ios-orange/10 rounded-2xl rounded-bl-md px-4 py-2.5 max-w-[85%] shadow-sm",
                div {
                    class: "text-sm text-stone-900 leading-relaxed break-words prose prose-sm",
                    dangerous_inner_html: md_to_html(&text),
                }
                if !sources.is_empty() {
                    SourcesAccordion {
                        sources: sources.clone(),
                        conversation_id: conversation_id,
                    }
                }
            }
        }
    }
}
