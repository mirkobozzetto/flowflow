use super::ai_card::{
    Brand, KeyBadge, ProviderCard, StatusConnected, StatusMuted, UseForChat,
};
use crate::application::chat_source;
use crate::application::i18n::t;
use crate::infrastructure::chatgpt_auth::{self, DeviceLogin};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::Provider;
use crate::ui::kit::{
    BTN_PRIMARY, INPUT, PILL_GHOST, PILL_PRIMARY, SECTION_LABEL,
};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

const KEY: &str = "openai_api_key";

#[derive(Clone)]
enum Connection {
    Disconnected,
    Pending(DeviceLogin),
    Connected,
}

/// OpenAI: the ChatGPT subscription and the API key, which also feeds the
/// note memory whatever answers the chat.
#[component]
pub fn OpenAiSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut active = use_signal(|| chat_source::active(&db()));
    let mut key = use_signal(|| db().get_setting(KEY).unwrap_or_default());
    let mut key_ready =
        use_signal(|| chat_source::is_ready(&db(), Provider::OpenAi));
    let mut saved = use_signal(|| false);
    let mut connection = use_signal(|| {
        if chatgpt_auth::is_connected() {
            Connection::Connected
        } else {
            Connection::Disconnected
        }
    });
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let connected = matches!(connection(), Connection::Connected);

    let connect = move |_| {
        error.set(None);
        spawn(async move {
            let login = match chatgpt_auth::begin_device_login().await {
                Ok(login) => login,
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            };
            connection.set(Connection::Pending(login.clone()));
            match chatgpt_auth::poll_device_login(&login).await {
                Ok(()) => {
                    let _ = chat_source::choose(&db(), Provider::ChatGpt);
                    active.set(Provider::ChatGpt);
                    connection.set(Connection::Connected);
                }
                Err(e) => {
                    error.set(Some(e));
                    connection.set(Connection::Disconnected);
                }
            }
        });
    };
    let disconnect = move |_| {
        chatgpt_auth::disconnect();
        let _ = chat_source::fall_back_from(&db(), Provider::ChatGpt);
        active.set(chat_source::active(&db()));
        connection.set(Connection::Disconnected);
    };

    rsx! {
        div { class: "space-y-6 pb-20",
            div {
                p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), {t(&lang, "settings-source-subscription")} }
                ProviderCard {
                    brand: Brand::OpenAi,
                    title: t(&lang, "settings-chat-auth-chatgpt"),
                    selected: connected && active() == Provider::ChatGpt,
                    status: match connection() {
                        Connection::Connected => rsx! {
                            StatusConnected { text: t(&lang, "settings-chatgpt-connected") }
                        },
                        Connection::Pending(_) => rsx! {
                            StatusMuted { text: t(&lang, "settings-chatgpt-pending"), pending: true }
                        },
                        Connection::Disconnected => rsx! {
                            StatusMuted { text: t(&lang, "settings-chatgpt-disconnected") }
                        },
                    },
                    right: match connection() {
                        Connection::Connected => rsx! {
                            button { class: PILL_GHOST, onclick: disconnect,
                                {t(&lang, "settings-chatgpt-disconnect")}
                            }
                        },
                        Connection::Pending(_) => rsx! {},
                        Connection::Disconnected => rsx! {
                            button { class: PILL_PRIMARY, onclick: connect,
                                {t(&lang, "settings-claude-connect")}
                            }
                        },
                    },
                    if let Connection::Pending(login) = connection() {
                        div { class: "mt-3 space-y-2 border-t border-stone-200/70 pt-3",
                            p { class: "text-sm text-stone-600", {t(&lang, "settings-chatgpt-code-hint")} }
                            a {
                                class: "block break-all text-sm font-medium text-ios-orange-dark underline",
                                href: login.verify_url.clone(),
                                target: "_blank",
                                "{login.verify_url}"
                            }
                            code { class: "block select-all rounded-lg bg-stone-100 px-3 py-2 text-center text-base font-semibold text-stone-900",
                                "{login.user_code}"
                            }
                        }
                    }
                    if let Some(error) = error() {
                        p { class: "mt-3 text-xs text-red-600", "{error}" }
                    }
                    UseForChat { source: Provider::ChatGpt, ready: connected, active }
                }
            }
            div {
                p { class: format!("{SECTION_LABEL} mb-1.5 ml-3"), {t(&lang, "settings-source-api-key")} }
                ProviderCard {
                    brand: Brand::OpenAi,
                    title: t(&lang, "settings-openai-label"),
                    selected: active() == Provider::OpenAi,
                    status: rsx! { StatusMuted { text: t(&lang, "settings-openai-chat-memory") } },
                    right: rsx! { KeyBadge { configured: key_ready() } },
                    input {
                        class: format!("{INPUT} mt-3"),
                        r#type: "password",
                        placeholder: "sk-...",
                        value: "{key}",
                        oninput: move |evt| {
                            key.set(evt.value());
                            saved.set(false);
                        },
                    }
                    p { class: "mt-2 text-xs text-stone-400", {t(&lang, "settings-openai-memory-hint")} }
                    UseForChat { source: Provider::OpenAi, ready: key_ready(), active }
                }
            }
            button {
                class: BTN_PRIMARY,
                onclick: move |_| {
                    let value = key().trim().to_string();
                    if !value.is_empty() {
                        let _ = db().set_setting(KEY, &value);
                    }
                    key_ready.set(chat_source::is_ready(&db(), Provider::OpenAi));
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
