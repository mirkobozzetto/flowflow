use crate::application::i18n::t;
use crate::infrastructure::chatgpt_auth::{self, DeviceLogin};
use crate::infrastructure::persistence::Database;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
enum ConnectionState {
    Disconnected,
    Pending(DeviceLogin),
    Connected,
}

#[component]
pub fn IntelligenceSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let mut openai_key =
        use_signal(|| db().get_setting("openai_api_key").unwrap_or_default());
    let mut exa_key =
        use_signal(|| db().get_setting("exa_api_key").unwrap_or_default());
    let mut max_sources = use_signal(|| {
        db().get_setting("rag_max_sources")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(8)
    });
    let mut saved = use_signal(|| false);
    let mut debug_trace =
        use_signal(|| db().get_setting("debug_trace").as_deref() == Some("1"));
    let mut chatgpt_selected = use_signal(|| {
        db().get_setting("llm_provider").as_deref() == Some("chatgpt")
    });
    let mut connection = use_signal(|| {
        if chatgpt_auth::is_connected() {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        }
    });
    let mut connection_error: Signal<Option<String>> = use_signal(|| None);
    let lang = (app.current_lang)();

    rsx! {
        div { class: "space-y-6 pb-20",
            h2 { class: "text-lg font-semibold text-stone-900",
                {t(&lang, "settings-chat-auth-title")}
            }
            div { class: "grid grid-cols-2 gap-1 rounded-xl bg-stone-100 p-1",
                button {
                    class: if !chatgpt_selected() {
                        "min-h-[40px] rounded-lg bg-white text-sm font-medium text-stone-900 shadow-sm"
                    } else {
                        "min-h-[40px] rounded-lg text-sm font-medium text-stone-500"
                    },
                    onclick: move |_| {
                        chatgpt_selected.set(false);
                        let _ = db().set_setting("llm_provider", "openai");
                    },
                    {t(&lang, "settings-chat-auth-api-key")}
                }
                button {
                    class: if chatgpt_selected() {
                        "min-h-[40px] rounded-lg bg-white text-sm font-medium text-stone-900 shadow-sm"
                    } else {
                        "min-h-[40px] rounded-lg text-sm font-medium text-stone-500"
                    },
                    onclick: move |_| {
                        chatgpt_selected.set(true);
                        let _ = db().set_setting("llm_provider", "chatgpt");
                    },
                    {t(&lang, "settings-chat-auth-chatgpt")}
                }
            }

            if chatgpt_selected() {
                div { class: "rounded-xl border border-stone-200 p-4 space-y-3",
                    match connection() {
                        ConnectionState::Disconnected => rsx! {
                            button {
                                class: crate::ui::kit::BTN_PRIMARY,
                                onclick: move |_| {
                                    connection_error.set(None);
                                    spawn(async move {
                                        match chatgpt_auth::begin_device_login().await {
                                            Ok(login) => {
                                                connection.set(ConnectionState::Pending(login.clone()));
                                                match chatgpt_auth::poll_device_login(&login).await {
                                                    Ok(()) => connection.set(ConnectionState::Connected),
                                                    Err(error) => {
                                                        connection_error.set(Some(error));
                                                        connection.set(ConnectionState::Disconnected);
                                                    }
                                                }
                                            }
                                            Err(error) => {
                                                connection_error.set(Some(error));
                                                connection.set(ConnectionState::Disconnected);
                                            }
                                        }
                                    });
                                },
                                {t(&lang, "settings-chatgpt-connect")}
                            }
                        },
                        ConnectionState::Pending(login) => rsx! {
                            p { class: "text-sm text-stone-600",
                                {t(&lang, "settings-chatgpt-code-hint")}
                            }
                            a {
                                class: "block break-all text-sm font-medium text-ios-orange underline",
                                href: login.verify_url.clone(),
                                target: "_blank",
                                "{login.verify_url}"
                            }
                            code { class: "block select-all rounded-lg bg-stone-100 px-3 py-2 text-center text-base font-semibold text-stone-900",
                                "{login.user_code}"
                            }
                        },
                        ConnectionState::Connected => rsx! {
                            div { class: "flex items-center justify-between gap-3",
                                p { class: "text-sm font-medium text-stone-700",
                                    {t(&lang, "settings-chatgpt-connected")}
                                }
                                button {
                                    class: "min-h-[36px] rounded-lg border border-stone-200 px-3 text-sm font-medium text-stone-700 active:bg-stone-100",
                                    onclick: move |_| {
                                        chatgpt_auth::disconnect();
                                        connection_error.set(None);
                                        connection.set(ConnectionState::Disconnected);
                                    },
                                    {t(&lang, "settings-chatgpt-disconnect")}
                                }
                            }
                        },
                    }
                    if let Some(error) = connection_error() {
                        p { class: "text-xs text-red-600", "{error}" }
                    }
                }
            }

            h2 { class: "text-lg font-semibold text-stone-900 pt-2",
                {t(&lang, "settings-api-keys-title")}
            }
            div {
                label { class: "block text-sm font-medium text-stone-700 mb-1", {t(&lang, "settings-openai-label")} }
                input {
                    class: crate::ui::kit::INPUT,
                    r#type: "password",
                    placeholder: "sk-...",
                    value: "{openai_key}",
                    oninput: move |evt| {
                        openai_key.set(evt.value());
                        saved.set(false);
                    },
                }
            }

            if chatgpt_selected() {
                p { class: "text-xs text-stone-400",
                    {t(&lang, "settings-chatgpt-embed-hint")}
                }
            }
            h2 { class: "text-lg font-semibold text-stone-900 pt-2",
                {t(&lang, "settings-search-title")}
            }
            div {
                label { class: "block text-sm font-medium text-stone-700 mb-1", {t(&lang, "settings-exa-label")} }
                input {
                    class: crate::ui::kit::INPUT,
                    r#type: "password",
                    placeholder: t(&lang, "settings-exa-placeholder"),
                    value: "{exa_key}",
                    oninput: move |evt| {
                        exa_key.set(evt.value());
                        saved.set(false);
                    },
                }
            }
            p { class: "text-xs text-stone-400",
                {t(&lang, "settings-web-search-moved")}
            }
            div {
                div { class: "flex justify-between items-center mb-1",
                    label { class: "text-sm font-medium text-stone-700",
                        {t(&lang, "settings-max-sources")}
                    }
                    span { class: "text-sm font-semibold text-ios-orange",
                        "{max_sources}"
                    }
                }
                input {
                    class: "w-full accent-ios-orange",
                    r#type: "range",
                    min: "3",
                    max: "15",
                    value: "{max_sources}",
                    oninput: move |evt| {
                        if let Ok(v) = evt.value().parse::<i64>() {
                            max_sources.set(v);
                            saved.set(false);
                        }
                    },
                }
                div { class: "flex justify-between text-xs text-stone-400",
                    span { "3" }
                    span { "15" }
                }
            }

            h2 { class: "text-lg font-semibold text-stone-900 pt-2", {t(&lang, "settings-debug-title")} }
            button {
                class: "w-full flex items-center justify-between gap-3 text-left",
                onclick: move |_| {
                    let on = !debug_trace();
                    debug_trace.set(on);
                    let _ = db().set_setting("debug_trace", if on { "1" } else { "0" });
                },
                div {
                    p { class: "text-sm font-medium text-stone-700", {t(&lang, "settings-debug-trace")} }
                    p { class: "text-xs text-stone-400", {t(&lang, "settings-debug-trace-desc")} }
                }
                span {
                    class: if debug_trace() {
                        "relative w-11 h-6 shrink-0 rounded-full bg-ios-orange transition-colors duration-200"
                    } else {
                        "relative w-11 h-6 shrink-0 rounded-full bg-stone-300 transition-colors duration-200"
                    },
                    span {
                        class: if debug_trace() {
                            "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-5"
                        } else {
                            "absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 translate-x-0"
                        },
                    }
                }
            }

            button {
                class: if saved() { crate::ui::kit::BTN_SUCCESS } else { crate::ui::kit::BTN_PRIMARY },
                onclick: move |_| {
                    let ok = openai_key().trim().to_string();
                    if !ok.is_empty() {
                        let _ = db().set_setting("openai_api_key", &ok);
                    }
                    let ek = exa_key().trim().to_string();
                    if !ek.is_empty() {
                        let _ = db().set_setting("exa_api_key", &ek);
                    }
                    let _ = db().set_setting(
                        "rag_max_sources",
                        &max_sources().to_string(),
                    );
                    saved.set(true);
                },
                if saved() {
                    {t(&lang, "settings-saved")}
                } else {
                    {t(&lang, "settings-save")}
                }
            }

            p { class: "text-xs text-stone-400 text-center",
                {t(&lang, "settings-keys-stored-locally")}
            }
        }
    }
}
