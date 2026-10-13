use super::ai_card::{Brand, KeyBadge, ProviderCard, StatusMuted, UseForChat};
use crate::application::chat_source;
use crate::application::i18n::t;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::Provider;
use crate::ui::kit::{BTN_PRIMARY, INPUT, SECTION_LABEL};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

const KEY: &str = "anthropic_api_key";

/// Claude: the subscription (test builds) and the API key, one level under AI.
#[component]
pub fn ClaudeSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let active = use_signal(|| chat_source::active(&db()));
    let mut key = use_signal(|| db().get_setting(KEY).unwrap_or_default());
    let mut key_ready =
        use_signal(|| chat_source::is_ready(&db(), Provider::Anthropic));
    let mut saved = use_signal(|| false);

    rsx! {
        div { class: "space-y-6 pb-20",
            {subscription(&lang, active)}
            div {
                p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), {t(&lang, "settings-source-api-key")} }
                ProviderCard {
                    brand: Brand::Claude,
                    title: t(&lang, "settings-claude-api-key"),
                    selected: active() == Provider::Anthropic,
                    status: rsx! { StatusMuted { text: t(&lang, "settings-chat-only") } },
                    right: rsx! { KeyBadge { configured: key_ready() } },
                    input {
                        class: format!("{INPUT} mt-3"),
                        r#type: "password",
                        placeholder: "sk-ant-…",
                        value: "{key}",
                        oninput: move |evt| {
                            key.set(evt.value());
                            saved.set(false);
                        },
                    }
                    UseForChat { source: Provider::Anthropic, ready: key_ready(), active }
                }
            }
            p { class: "text-xs text-stone-400", {t(&lang, "settings-memory-openai-hint")} }
            button {
                class: BTN_PRIMARY,
                onclick: move |_| {
                    let value = key().trim().to_string();
                    if !value.is_empty() {
                        let _ = db().set_setting(KEY, &value);
                    }
                    key_ready.set(chat_source::is_ready(&db(), Provider::Anthropic));
                    saved.set(true);
                },
                if saved() {
                    {t(&lang, "settings-saved")}
                } else {
                    {t(&lang, "settings-save")}
                }
            }
        }
    }
}

#[cfg(debug_assertions)]
fn subscription(lang: &str, active: Signal<Provider>) -> Element {
    rsx! {
        div {
            p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), {t(lang, "settings-source-subscription")} }
            super::claude_subscription_card::ClaudeSubscriptionCard { active }
        }
    }
}

#[cfg(not(debug_assertions))]
fn subscription(_lang: &str, _active: Signal<Provider>) -> Element {
    rsx! {}
}
