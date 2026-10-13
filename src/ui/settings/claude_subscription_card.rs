//! Claude Pro/Max sign-in card: test builds only (infrastructure/mod.rs).
use super::ai_card::{
    Brand, ProviderCard, StatusConnected, StatusMuted, UseForChat,
};
use crate::application::chat_source;
use crate::application::i18n::t;
use crate::infrastructure::claude_auth::{self, ClaudeLogin};
use crate::infrastructure::persistence::Database;
use crate::infrastructure::{platform, Provider};
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
enum Connection {
    Disconnected,
    Pending(ClaudeLogin),
    Connected,
}

#[component]
pub fn ClaudeSubscriptionCard(active: Signal<Provider>) -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();
    let mut connection = use_signal(|| {
        if claude_auth::is_connected() {
            Connection::Connected
        } else {
            Connection::Disconnected
        }
    });
    let mut code = use_signal(String::new);
    let mut refused = use_signal(|| false);
    let mut checking = use_signal(|| false);
    let connected = connection() == Connection::Connected;

    let connect = move |_| match claude_auth::begin_login() {
        Ok(login) => {
            platform::open_url(&login.authorize_url);
            code.set(String::new());
            refused.set(false);
            connection.set(Connection::Pending(login));
        }
        Err(error) => {
            eprintln!("[claude] sign-in link failed: {error}");
            refused.set(true);
        }
    };
    let validate = move |_| {
        let Connection::Pending(login) = connection() else {
            return;
        };
        if code().trim().is_empty() || checking() {
            return;
        }
        checking.set(true);
        spawn(async move {
            match claude_auth::complete_login(&login, &code()).await {
                Ok(()) => {
                    eprintln!("[claude] connected");
                    let _ = chat_source::choose(
                        &db(),
                        Provider::ClaudeSubscription,
                    );
                    active.set(Provider::ClaudeSubscription);
                    connection.set(Connection::Connected);
                }
                Err(error) => {
                    eprintln!("[claude] code refused: {error}");
                    refused.set(true);
                    connection.set(Connection::Disconnected);
                }
            }
            checking.set(false);
        });
    };
    let disconnect = move |_| {
        claude_auth::disconnect();
        eprintln!("[claude] disconnected");
        let _ =
            chat_source::fall_back_from(&db(), Provider::ClaudeSubscription);
        active.set(chat_source::active(&db()));
        connection.set(Connection::Disconnected);
    };

    rsx! {
        ProviderCard {
            brand: Brand::Claude,
            title: t(&lang, "settings-claude-subscription"),
            selected: connected && active() == Provider::ClaudeSubscription,
            status: match connection() {
                Connection::Connected => rsx! {
                    StatusConnected { text: t(&lang, "settings-claude-connected") }
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
                    button { class: crate::ui::kit::PILL_GHOST, onclick: disconnect,
                        {t(&lang, "settings-chatgpt-disconnect")}
                    }
                },
                Connection::Pending(_) => rsx! {},
                Connection::Disconnected => rsx! {
                    button { class: crate::ui::kit::PILL_PRIMARY, onclick: connect,
                        {t(&lang, "settings-claude-connect")}
                    }
                },
            },
            if let Connection::Pending(login) = connection() {
                div { class: "mt-3 space-y-2 border-t border-stone-200/70 pt-3",
                    p { class: "text-sm text-stone-600", {t(&lang, "settings-claude-code-hint")} }
                    button {
                        class: "block text-sm font-medium text-ios-orange-dark underline",
                        onclick: move |_| platform::open_url(&login.authorize_url),
                        {t(&lang, "settings-claude-open")}
                    }
                    div { class: "flex items-center gap-2",
                        input {
                            class: format!("{} min-w-0 font-mono", crate::ui::kit::INPUT),
                            r#type: "text",
                            autocomplete: "off",
                            autocapitalize: "off",
                            spellcheck: "false",
                            placeholder: t(&lang, "settings-claude-code-placeholder"),
                            value: "{code}",
                            oninput: move |evt| code.set(evt.value()),
                        }
                        button {
                            class: crate::ui::kit::PILL_PRIMARY,
                            disabled: code().trim().is_empty() || checking(),
                            onclick: validate,
                            {t(&lang, "settings-claude-validate")}
                        }
                    }
                }
            }
            if refused() {
                p { class: "mt-3 text-xs text-red-600", {t(&lang, "settings-claude-code-error")} }
            }
            UseForChat { source: Provider::ClaudeSubscription, ready: connected, active }
        }
    }
}
