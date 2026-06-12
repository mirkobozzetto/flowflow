use crate::services::i18n::t;
use crate::ui::chat::actions::md_to_html;
use crate::ui::chat::models::ChatSource;
use crate::ui::chat::sources_accordion::SourcesAccordion;
use crate::ui::clipboard::copy_text;
use crate::ui::icons::{IconCheck, IconCopy};
use crate::ui::AppState;
use dioxus::prelude::*;

#[component]
pub fn BotBubble(
    text: String,
    sources: Vec<ChatSource>,
    conversation_id: Option<String>,
) -> Element {
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut copied = use_signal(|| false);
    let copy_label = t(&lang, "chat-copy");
    let copied_label = t(&lang, "chat-copied");
    let text_for_copy = text.clone();

    rsx! {
        div {
            class: "flex justify-start",
            style: "animation: fadeInUp 0.15s ease-out;",
            div { class: "bg-warm-white border border-ios-orange/10 rounded-2xl rounded-bl-md px-4 py-2.5 max-w-[85%] shadow-sm",
                div {
                    class: "text-sm text-stone-900 leading-relaxed break-words prose prose-sm",
                    dangerous_inner_html: md_to_html(&text),
                }
                div { class: "flex justify-end mt-1.5",
                    button {
                        class: if copied() {
                            "flex items-center gap-1 text-xs text-ios-green transition-colors duration-150"
                        } else {
                            "flex items-center gap-1 text-xs text-stone-400 active:text-stone-600 hover:text-stone-600 transition-colors duration-150"
                        },
                        onclick: move |_| {
                            if copied() {
                                return;
                            }
                            copy_text(&text_for_copy);
                            copied.set(true);
                            spawn(async move {
                                futures_timer::Delay::new(
                                    std::time::Duration::from_millis(1500),
                                )
                                .await;
                                copied.set(false);
                            });
                        },
                        if copied() {
                            IconCheck { size: 12 }
                            "{copied_label}"
                        } else {
                            IconCopy { size: 12 }
                            "{copy_label}"
                        }
                    }
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
