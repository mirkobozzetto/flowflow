use crate::application::i18n::t;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::persistence::Database;
use crate::ui::icons::*;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn TagsSection(
    tags: Signal<Vec<String>>,
    tag_input: Signal<String>,
    tagging: Signal<bool>,
    content: Signal<String>,
) -> Element {
    let app: AppState = use_context();
    let db: Signal<Arc<Database>> = use_context();
    let lang = (app.current_lang)();
    let auto_tag_label = t(&lang, "tag-auto-action");
    let placeholder = t(&lang, "tag-add-placeholder");
    let mut tags = tags;
    let mut tag_input = tag_input;
    let mut tagging = tagging;

    rsx! {
        div { class: "flex flex-wrap items-center gap-1.5 mb-2",
            if tags().is_empty() {
                button {
                    class: if tagging() {
                        "px-2.5 py-1 rounded-full bg-stone-100 text-stone-400 text-xs"
                    } else {
                        "px-2.5 py-1 rounded-full bg-stone-200 text-stone-600 text-xs font-medium"
                    },
                    disabled: tagging() || content().trim().len() < 20,
                    onclick: move |_| {
                        tagging.set(true);
                        let c = content();
                        spawn(async move {
                            if let Ok(client) = LlmClient::from_db(&db()) {
                                if let Ok(new_tags) =
                                    client.generate_tags(&c).await
                                {
                                    let mut current = tags();
                                    for t in new_tags {
                                        if !current.contains(&t) {
                                            current.push(t);
                                        }
                                    }
                                    tags.set(current);
                                }
                            }
                            tagging.set(false);
                        });
                    },
                    {
                        let label = if tagging() { "...".to_string() } else { auto_tag_label.clone() };
                        rsx! { "{label}" }
                    }
                }
            }
            for (i, tag) in tags().iter().enumerate() {
                {
                    let tag_display = tag.clone();
                    rsx! {
                        span {
                            key: "{i}",
                            class: "inline-flex items-center gap-1 px-2.5 py-1 rounded-full bg-warm-white border border-ios-orange/25 text-ios-orange-dark text-xs font-medium",
                            "{tag_display}"
                            button {
                                class: "ml-0.5 text-ios-orange-dark hover:text-ios-orange-dark",
                                onclick: move |_| {
                                    let mut t = tags();
                                    t.remove(i);
                                    tags.set(t);
                                },
                                IconX { size: 12 }
                            }
                        }
                    }
                }
            }
            if tags().len() < 3 {
                div { class: "inline-flex items-center gap-1",
                    input {
                        class: "w-24 text-xs border border-stone-200 rounded-full px-2.5 py-1 outline-none focus:border-ios-orange-dark transition-colors duration-150",
                        placeholder: "{placeholder}",
                        value: "{tag_input}",
                        oninput: move |evt| tag_input.set(evt.value()),
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter {
                                let v = tag_input().trim().to_string();
                                if !v.is_empty() && !tags().contains(&v) {
                                    let mut t = tags();
                                    t.push(v);
                                    tags.set(t);
                                    tag_input.set(String::new());
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
