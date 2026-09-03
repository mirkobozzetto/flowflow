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
    let openai_configured = !openai_key().trim().is_empty();

    rsx! {
        div { class: "space-y-6 pb-20",
            p { class: crate::ui::kit::SECTION_LABEL,
                {t(&lang, "settings-chat-role-title")}
            }
            div {
                class: if chatgpt_selected() {
                    "rounded-xl border border-ios-orange-100 bg-warm-white p-4 shadow-card"
                } else {
                    "rounded-xl border border-stone-200 bg-warm-white p-4"
                },
                div { class: "flex min-w-0 items-center gap-3",
                    div { class: "grid h-11 w-11 shrink-0 place-items-center rounded-xl border border-stone-200 bg-white text-stone-900",
                        OpenAiLogo {}
                    }
                    div { class: "min-w-0 flex-1",
                        p { class: "truncate text-[15px] font-semibold text-stone-900",
                            {t(&lang, "settings-chat-auth-chatgpt")}
                        }
                        match connection() {
                            ConnectionState::Connected => rsx! {
                                div { class: "mt-1 flex items-center gap-2 text-xs font-semibold text-ios-orange-dark",
                                    span { class: "h-2 w-2 shrink-0 rounded-full bg-ios-orange ring-[3px] ring-ios-orange-50" }
                                    span { {t(&lang, "settings-chatgpt-connected")} }
                                }
                            },
                            ConnectionState::Pending(_) => rsx! {
                                p { class: "mt-1 text-xs font-medium text-ios-orange-dark",
                                    {t(&lang, "settings-chatgpt-pending")}
                                }
                            },
                            ConnectionState::Disconnected => rsx! {
                                p { class: "mt-1 text-xs text-stone-500",
                                    {t(&lang, "settings-chatgpt-disconnected")}
                                }
                            },
                        }
                    }
                    match connection() {
                        ConnectionState::Connected if chatgpt_selected() => rsx! {
                            button {
                                class: crate::ui::kit::PILL_GHOST,
                                onclick: move |_| {
                                    chatgpt_auth::disconnect();
                                    connection_error.set(None);
                                    connection.set(ConnectionState::Disconnected);
                                },
                                {t(&lang, "settings-chatgpt-disconnect")}
                            }
                        },
                        ConnectionState::Connected => rsx! {
                            button {
                                class: crate::ui::kit::PILL_PRIMARY,
                                onclick: move |_| {
                                    chatgpt_selected.set(true);
                                    let _ = db().set_setting("llm_provider", "chatgpt");
                                },
                                {t(&lang, "settings-chat-use-chatgpt")}
                            }
                        },
                        ConnectionState::Pending(_) => rsx! {},
                        ConnectionState::Disconnected => rsx! {
                            button {
                                class: crate::ui::kit::PILL_PRIMARY,
                                onclick: move |_| {
                                    chatgpt_selected.set(true);
                                    let _ = db().set_setting("llm_provider", "chatgpt");
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
                    }
                }

                if chatgpt_selected() {
                    div { class: "mt-3 flex min-w-0 items-center justify-between gap-3 rounded-xl bg-stone-50 px-3 py-2.5",
                        div { class: "min-w-0",
                            p { class: "text-[11px] font-medium text-stone-500",
                                {t(&lang, "settings-chat-model-current")}
                            }
                            p { class: "truncate text-sm font-semibold text-stone-800",
                                {crate::application::constants::CHATGPT_CHAT_MODEL_NAME}
                            }
                        }
                        span { class: "shrink-0 rounded-full bg-ios-orange-50 px-2.5 py-1 text-[11px] font-bold text-ios-orange-dark",
                            {t(&lang, "settings-reasoning-medium")}
                        }
                    }
                }

                if let ConnectionState::Pending(login) = connection() {
                    div { class: "mt-3 space-y-2 border-t border-stone-200/70 pt-3",
                        p { class: "text-sm text-stone-600",
                            {t(&lang, "settings-chatgpt-code-hint")}
                        }
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

                if chatgpt_selected() && openai_configured {
                    button {
                        class: "mt-3 text-xs font-semibold text-ios-orange-dark",
                        onclick: move |_| {
                            chatgpt_selected.set(false);
                            let _ = db().set_setting("llm_provider", "openai");
                        },
                        {t(&lang, "settings-chat-use-openai")}
                    }
                }

                if let Some(error) = connection_error() {
                    p { class: "mt-3 text-xs text-red-600", "{error}" }
                }
            }

            p { class: "pt-2 text-xs font-medium uppercase tracking-wide text-stone-400",
                {t(&lang, "settings-memory-search-title")}
            }
            div { class: "rounded-xl border border-stone-200 bg-warm-white p-4",
                div { class: "flex min-w-0 items-center gap-3",
                    div { class: "grid h-11 w-11 shrink-0 place-items-center rounded-xl border border-stone-200 bg-white text-stone-900",
                        OpenAiLogo {}
                    }
                    div { class: "min-w-0 flex-1",
                        p { class: "truncate text-[15px] font-semibold text-stone-900",
                            {t(&lang, "settings-openai-label")}
                        }
                        p { class: "mt-0.5 truncate text-xs text-stone-500",
                            if chatgpt_selected() {
                                {t(&lang, "settings-openai-embeddings-only")}
                            } else {
                                {t(&lang, "settings-openai-chat-embeddings")}
                            }
                        }
                    }
                    span {
                        class: if openai_configured {
                            "shrink-0 rounded-full bg-ios-orange-50 px-2 py-1 text-[10px] font-bold text-ios-orange-dark"
                        } else {
                            "shrink-0 rounded-full bg-stone-100 px-2 py-1 text-[10px] font-bold text-stone-500"
                        },
                        if openai_configured {
                            {t(&lang, "settings-key-configured")}
                        } else {
                            {t(&lang, "settings-key-missing")}
                        }
                    }
                }
                input {
                    class: "mt-3 w-full bg-stone-50 border border-stone-200 rounded-lg px-3 py-2.5 text-sm outline-none text-stone-900 placeholder-stone-400 focus:border-ios-orange-dark focus:ring-[3px] focus:ring-ios-orange-50 transition-colors duration-150",
                    r#type: "password",
                    placeholder: "sk-...",
                    value: "{openai_key}",
                    oninput: move |evt| {
                        openai_key.set(evt.value());
                        saved.set(false);
                    },
                }
                if chatgpt_selected() {
                    p { class: "mt-2 text-xs text-stone-400",
                        {t(&lang, "settings-chatgpt-embed-hint")}
                    }
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
                class: crate::ui::kit::BTN_PRIMARY,
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

const OPENAI_BLOSSOM_PATH: &str = "M249.176 323.434V298.276C249.176 296.158 249.971 294.569 251.825 293.509L302.406 264.381C309.29 260.409 317.5 258.555 325.973 258.555C357.75 258.555 377.877 283.185 377.877 309.399C377.877 311.253 377.877 313.371 377.611 315.49L325.178 284.771C322.001 282.919 318.822 282.919 315.645 284.771L249.176 323.434ZM367.283 421.415V361.301C367.283 357.592 365.694 354.945 362.516 353.092L296.048 314.43L317.763 301.982C319.617 300.925 321.206 300.925 323.058 301.982L373.639 331.112C388.205 339.586 398.003 357.592 398.003 375.069C398.003 395.195 386.087 413.733 367.283 421.412V421.415ZM233.553 368.452L211.838 355.742C209.986 354.684 209.19 353.095 209.19 350.975V292.718C209.19 264.383 230.905 242.932 260.301 242.932C271.423 242.932 281.748 246.641 290.49 253.26L238.321 283.449C235.146 285.303 233.555 287.951 233.555 291.659V368.455L233.553 368.452ZM280.292 395.462L249.176 377.985V340.913L280.292 323.436L311.407 340.913V377.985L280.292 395.462ZM300.286 475.968C289.163 475.968 278.837 472.259 270.097 465.64L322.264 435.449C325.441 433.597 327.03 430.949 327.03 427.239V350.445L349.011 363.155C350.865 364.213 351.66 365.802 351.66 367.922V426.179C351.66 454.514 329.679 475.965 300.286 475.965V475.968ZM237.525 416.915L186.944 387.785C172.378 379.31 162.582 361.305 162.582 343.827C162.582 323.436 174.763 305.164 193.563 297.485V357.861C193.563 361.571 195.154 364.217 198.33 366.071L264.535 404.467L242.82 416.915C240.967 417.972 239.377 417.972 237.525 416.915ZM234.614 460.343C204.689 460.343 182.71 437.833 182.71 410.028C182.71 407.91 182.976 405.792 183.238 403.672L235.405 433.863C238.582 435.715 241.763 435.715 244.938 433.863L311.407 395.466V420.622C311.407 422.742 310.612 424.331 308.758 425.389L258.179 454.519C251.293 458.491 243.083 460.343 234.611 460.343H234.614ZM300.286 491.854C332.329 491.854 359.073 469.082 365.167 438.892C394.825 431.211 413.892 403.406 413.892 375.073C413.892 356.535 405.948 338.529 391.648 325.552C392.972 319.991 393.766 314.43 393.766 308.87C393.766 271.003 363.048 242.666 327.562 242.666C320.413 242.666 313.528 243.723 306.644 246.109C294.725 234.457 278.307 227.042 260.301 227.042C228.258 227.042 201.513 249.815 195.42 280.004C165.761 287.685 146.694 315.49 146.694 343.824C146.694 362.362 154.638 380.368 168.938 393.344C167.613 398.906 166.819 404.467 166.819 410.027C166.819 447.894 197.538 476.231 233.024 476.231C240.172 476.231 247.058 475.173 253.943 472.788C265.859 484.441 282.278 491.854 300.286 491.854Z";

#[component]
fn OpenAiLogo() -> Element {
    rsx! {
        svg {
            width: "24",
            height: "24",
            view_box: "146 227 268 266",
            fill: "currentColor",
            path { d: OPENAI_BLOSSOM_PATH }
        }
    }
}
