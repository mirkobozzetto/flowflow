use crate::db::Database;
use crate::services::backend::{BackendClient, Connector};
use crate::services::i18n::t;
use crate::ui::AppState;
use dioxus::prelude::*;
use std::sync::Arc;
use std::time::Duration;

// When the backend URL is baked in at build time (FLOWFLOW_BACKEND_URL), users never type
// it - the manual field is shown only as a dev/self-host fallback when nothing is baked.
const BAKED_BACKEND_URL: Option<&str> = option_env!("FLOWFLOW_BACKEND_URL");

#[component]
pub fn ConnectionsSettings() -> Element {
    let db: Signal<Arc<Database>> = use_context();
    let app: AppState = use_context();
    let lang = (app.current_lang)();

    let device_id =
        use_signal(|| BackendClient::device_pubkey(&db()).unwrap_or_default());
    let mut base_url =
        use_signal(|| db().get_setting("backend_base_url").unwrap_or_default());
    let mut connectors = use_signal(Vec::<Connector>::new);
    let mut status: Signal<Option<String>> = use_signal(|| None);
    let mut busy = use_signal(|| false);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let _trigger = reload();
        spawn(async move {
            let database = db();
            match BackendClient::from_db(&database) {
                None => connectors.set(Vec::new()),
                Some(client) => match client.list_connectors(&database).await {
                    Ok(list) => {
                        connectors.set(list);
                        status.set(None);
                    }
                    Err(e) => status.set(Some(e.to_string())),
                },
            }
        });
    });

    rsx! {
        div { class: "space-y-6 pb-20",
            p { class: "text-xs text-stone-500 leading-relaxed",
                {t(&lang, "connections-description")}
            }

            if !device_id().is_empty() {
                div {
                    label { class: "block text-sm font-medium text-stone-700 mb-1",
                        {t(&lang, "connections-device-id-label")}
                    }
                    p { class: "text-[11px] text-stone-400 mb-2 leading-relaxed",
                        {t(&lang, "connections-device-id-hint")}
                    }
                    div { class: "flex items-center gap-2",
                        code {
                            class: "flex-1 text-[11px] font-mono break-all bg-warm-white border border-stone-200 rounded-lg px-3 py-2 text-stone-700",
                            "{device_id}"
                        }
                        button {
                            class: "shrink-0 h-9 px-3 flex items-center justify-center rounded-lg bg-stone-100 text-stone-900 text-xs font-medium hover:bg-stone-200 active:bg-stone-200 transition-colors duration-150",
                            onclick: move |_| crate::ui::clipboard::copy_text(&device_id()),
                            {t(&lang, "connections-copy")}
                        }
                    }
                }
            }

            if BAKED_BACKEND_URL.is_none() {
                div {
                    label { class: "block text-sm font-medium text-stone-700 mb-1",
                        {t(&lang, "connections-backend-url-label")}
                    }
                    input {
                        class: crate::ui::kit::INPUT,
                        r#type: "url",
                        placeholder: "https://...",
                        value: "{base_url}",
                        oninput: move |evt| base_url.set(evt.value()),
                    }
                    button {
                        class: format!("{} mt-2", crate::ui::kit::BTN_PRIMARY),
                        onclick: move |_| {
                            let _ = db().set_setting("backend_base_url", base_url().trim());
                            reload.set(reload() + 1);
                        },
                        {t(&lang, "connections-save-url")}
                    }
                }
            }

            if let Some(err) = status() {
                div { class: "rounded-xl border border-ios-red/40 bg-ios-red/5 p-3",
                    p { class: "text-xs text-stone-600 break-words", "{err}" }
                }
            }

            if connectors().is_empty() {
                p { class: "text-xs text-stone-400", {t(&lang, "connections-empty")} }
            }

            for c in connectors() {
                div {
                    key: "{c.provider}",
                    class: "rounded-xl bg-warm-white border border-stone-200 p-4 flex items-center justify-between",
                    div {
                        p { class: "text-sm font-medium text-stone-800", "{c.name}" }
                        p {
                            class: if c.connected { "text-xs text-ios-green" } else { "text-xs text-stone-400" },
                            if c.connected {
                                {t(&lang, "connections-connected")}
                            } else {
                                {t(&lang, "connections-not-connected")}
                            }
                        }
                    }
                    if c.connected {
                            button {
                                class: crate::ui::kit::CONFIRM_BTN_GHOST,
                                disabled: busy(),
                                onclick: {
                                    let provider = c.provider.clone();
                                    move |_| {
                                        if busy() { return; }
                                        busy.set(true);
                                        status.set(None);
                                        let provider = provider.clone();
                                        spawn(async move {
                                            let database = db();
                                            if let Some(client) = BackendClient::from_db(&database) {
                                                if let Err(e) = client.disconnect(&database, &provider).await {
                                                    status.set(Some(e.to_string()));
                                                }
                                            }
                                            busy.set(false);
                                            reload.set(reload() + 1);
                                        });
                                    }
                                },
                                {t(&lang, "connections-disconnect")}
                            }
                        } else {
                            button {
                                class: crate::ui::kit::CONFIRM_BTN_PRIMARY,
                                disabled: busy(),
                                onclick: {
                                    let provider = c.provider.clone();
                                    move |_| {
                                        if busy() { return; }
                                        busy.set(true);
                                        status.set(None);
                                        let provider = provider.clone();
                                        spawn(async move {
                                            let database = db();
                                            match connect_flow(&database, &provider).await {
                                                Ok(()) => {}
                                                Err(e) => status.set(Some(e)),
                                            }
                                            busy.set(false);
                                            reload.set(reload() + 1);
                                        });
                                    }
                                },
                                {t(&lang, "connections-connect")}
                            }
                        }
                }
            }
        }
    }
}

// authorize -> open the consent page in the system browser -> poll the backend until
// it reports the connector connected. The backend captures the OAuth callback at its own
// https redirect URI, so this path carries no custom scheme and is identical on every
// platform.
async fn connect_flow(db: &Database, provider: &str) -> Result<(), String> {
    let client = BackendClient::from_db(db)
        .ok_or_else(|| "no backend configured".to_string())?;
    let auth_url = client
        .authorize(db, provider)
        .await
        .map_err(|e| e.to_string())?;
    crate::platform::open_url(&auth_url);

    // ~5 min of foreground polling. iOS suspends the timer while the browser is up, so
    // this is wall-clock back in the app after consent, not real time.
    for _ in 0..150 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        // A blip while backgrounded is not a failure; only the timeout is.
        if let Ok(list) = client.list_connectors(db).await {
            if list.iter().any(|c| c.provider == provider && c.connected) {
                return Ok(());
            }
        }
    }
    Err("timed out waiting for the connection".to_string())
}
