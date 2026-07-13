use super::tab_pins::{load_tab_picker, pinned_tabs, TabPicker, TabPinEditor};
use crate::application::i18n::t;
use crate::infrastructure::backend::{BackendClient, Connector};
use crate::infrastructure::persistence::Database;
use crate::ui::icons::{IconCheck, IconCopy};
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
    // The agent directory: every published agent this account is entitled to.
    let mut agents =
        use_signal(Vec::<crate::application::agent_directory::AgentEntry>::new);
    let mut status: Signal<Option<String>> = use_signal(|| None);
    let mut busy = use_signal(|| false);
    let mut dev_copied = use_signal(|| false);
    let mut reload = use_signal(|| 0u32);
    // Arm-time binding: the sheets offered for selection, and the ones the agent is currently armed to.
    let mut sheets: Signal<
        Vec<crate::application::connector_module::Spreadsheet>,
    > = use_signal(Vec::new);
    let mut bindings = use_signal(|| {
        crate::application::connector_module::current_bindings(&db())
    });
    let mut listing = use_signal(|| false);
    // Per-tab pin editor: (spreadsheet_id, every tab, checked set). None = closed.
    let mut tab_picker: Signal<Option<TabPicker>> = use_signal(|| None);
    // The spreadsheet whose tabs are being fetched (disables its button meanwhile).
    let mut tabs_loading: Signal<Option<String>> = use_signal(|| None);
    // The agent-sheets accordion starts open when nothing is armed yet, to guide the first arming.
    let mut sheets_open = use_signal(|| {
        crate::application::connector_module::current_bindings(&db()).is_empty()
    });

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
        spawn(async move {
            // Best-effort: no backend or an offline start just leaves the list empty.
            if let Ok(list) =
                crate::application::agent_directory::list_agents(&db()).await
            {
                agents.set(list);
            }
        });
    });

    // Heal any armed sheet still showing its id (a legacy bind, or a URL bind made before the title was
    // resolvable) by resolving the real title in the background, once on mount.
    use_effect(move || {
        spawn(async move {
            let updated =
                crate::application::connector_module::resolve_missing_names(
                    &db(),
                )
                .await;
            bindings.set(updated);
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
                            class: if dev_copied() {
                                "shrink-0 p-1.5 rounded-md text-ios-green transition-colors"
                            } else {
                                "shrink-0 p-1.5 rounded-md text-stone-400 hover:bg-stone-100 active:bg-stone-100 transition-colors"
                            },
                            onclick: move |_| {
                                if dev_copied() { return; }
                                crate::ui::clipboard::copy_text(&device_id());
                                dev_copied.set(true);
                                spawn(async move {
                                    futures_timer::Delay::new(
                                        Duration::from_millis(1500),
                                    )
                                    .await;
                                    dev_copied.set(false);
                                });
                            },
                            if dev_copied() {
                                IconCheck { size: 16 }
                            } else {
                                IconCopy { size: 16 }
                            }
                        }
                    }
                }
            }

            if !agents().is_empty() {
                div {
                    label { class: "block text-sm font-medium text-stone-700 mb-1",
                        {t(&lang, "connections-agents-label")}
                    }
                    p { class: "text-[11px] text-stone-400 mb-2 leading-relaxed",
                        {t(&lang, "connections-agents-hint")}
                    }
                    div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                        for a in agents() {
                            div { key: "{a.id}", class: "px-4 py-3 min-h-[44px] flex items-center gap-3",
                                span {
                                    class: if a.installed {
                                        "w-1.5 h-1.5 shrink-0 rounded-full bg-ios-green"
                                    } else {
                                        "w-1.5 h-1.5 shrink-0 rounded-full bg-stone-300"
                                    },
                                }
                                div { class: "flex-1 min-w-0",
                                    div { class: "flex items-center gap-1.5 min-w-0",
                                        p { class: "text-sm font-medium text-stone-800 truncate", "{a.name}" }
                                        if a.stale {
                                            span { class: "shrink-0 text-[10px] font-medium text-stone-500 bg-stone-100 rounded-full px-1.5 py-0.5",
                                                {t(&lang, "connections-agent-stale")}
                                            }
                                        }
                                    }
                                    if !a.alias.is_empty() {
                                        p { class: "text-[11px] text-stone-400 truncate", "{a.alias}" }
                                    }
                                    // The id disambiguates same-named agents (display names are not unique).
                                    code { class: "block text-[10px] font-mono text-stone-400 truncate", "{a.id}" }
                                }
                                if a.installed {
                                    button {
                                        class: crate::ui::kit::PILL_GHOST,
                                        disabled: busy(),
                                        onclick: {
                                            let id = a.id.clone();
                                            move |_| {
                                                if busy() { return; }
                                                status.set(None);
                                                match crate::application::agent_directory::uninstall(&db(), &id) {
                                                    Ok(()) => reload.set(reload() + 1),
                                                    Err(e) => status.set(Some(e)),
                                                }
                                            }
                                        },
                                        {t(&lang, "connections-agent-remove")}
                                    }
                                } else {
                                    button {
                                        class: crate::ui::kit::PILL_PRIMARY,
                                        disabled: busy(),
                                        onclick: {
                                            let id = a.id.clone();
                                            move |_| {
                                                if busy() { return; }
                                                busy.set(true);
                                                status.set(None);
                                                let id = id.clone();
                                                spawn(async move {
                                                    match crate::application::agent_directory::ensure_installed(&db(), &id).await {
                                                        Ok(()) => reload.set(reload() + 1),
                                                        Err(e) => status.set(Some(e)),
                                                    }
                                                    busy.set(false);
                                                });
                                            }
                                        },
                                        {t(&lang, "connections-agent-install")}
                                    }
                                }
                            }
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
            } else {
                div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                    for c in connectors() {
                        div {
                            key: "{c.provider}",
                            class: "px-4 py-3",
                            div { class: "min-h-[44px] flex items-center gap-3",
                            // Real brand logo for known providers, first-letter fallback otherwise.
                            div { class: "w-10 h-10 shrink-0 rounded-xl bg-white border border-stone-200 flex items-center justify-center overflow-hidden",
                                {
                                    match c.provider.as_str() {
                                        "google" => rsx! { crate::ui::icons::IconGoogleSheets { size: 24 } },
                                        _ => rsx! {
                                            span { class: "text-stone-600 font-semibold text-base",
                                                {c.name.chars().next().map(|ch| ch.to_uppercase().to_string()).unwrap_or_default()}
                                            }
                                        },
                                    }
                                }
                            }
                            div { class: "flex-1 min-w-0",
                                p { class: "text-sm font-medium text-stone-800 truncate", "{c.name}" }
                                div { class: "flex items-center gap-1.5 mt-0.5 min-w-0",
                                    span {
                                        class: if c.connected {
                                            "w-1.5 h-1.5 shrink-0 rounded-full bg-ios-green"
                                        } else {
                                            "w-1.5 h-1.5 shrink-0 rounded-full bg-stone-300"
                                        },
                                    }
                                    span {
                                        class: if c.connected {
                                            "text-[11px] text-ios-green truncate"
                                        } else {
                                            "text-[11px] text-stone-400 truncate"
                                        },
                                        if c.connected {
                                            {t(&lang, "connections-connected")}
                                        } else {
                                            {t(&lang, "connections-not-connected")}
                                        }
                                    }
                                }
                            }
                            if c.connected {
                                button {
                                    class: crate::ui::kit::PILL_GHOST,
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
                                    class: crate::ui::kit::PILL_PRIMARY,
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
                            // Arm the agent to its spreadsheets (multi-bind), then validate a bound run.
                            if c.connected && c.provider == "google" {
                                div { class: "mt-3 pl-[52px]",
                                    button {
                                        class: "w-full flex items-center justify-between min-h-[40px] text-left",
                                        onclick: move |_| {
                                            let closing = sheets_open();
                                            sheets_open.set(!closing);
                                            // Closing also clears the loaded pick list, so reopening starts clean.
                                            if closing {
                                                sheets.set(Vec::new());
                                            }
                                        },
                                        div { class: "flex items-center gap-2",
                                            p { class: "text-[11px] font-medium text-stone-500", {t(&lang, "connections-agent-sheets")} }
                                            if !bindings().is_empty() {
                                                span { class: "text-[10px] font-medium text-ios-orange bg-ios-orange-50 rounded-full px-1.5 py-0.5", "{bindings().len()}" }
                                            }
                                        }
                                        span {
                                            class: "text-stone-400 text-sm leading-none",
                                            style: if sheets_open() {
                                                "transform: rotate(90deg); transition: transform 0.18s ease;"
                                            } else {
                                                "transition: transform 0.18s ease;"
                                            },
                                            "›"
                                        }
                                    }

                                    if sheets_open() {
                                        div { class: "mt-3 space-y-3", style: "animation: fadeInUp 0.18s ease-out;",

                                            if bindings().is_empty() {
                                                p { class: "text-xs text-stone-400", {t(&lang, "connections-no-sheet")} }
                                            } else {
                                                div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                                                    for (id, name) in bindings() {
                                                        div { key: "{id}", class: "p-3 space-y-2", style: "animation: fadeInUp 0.18s ease-out;",
                                                            div { class: "flex items-center gap-2",
                                                                span { class: "w-1.5 h-1.5 shrink-0 rounded-full bg-ios-green" }
                                                                p { class: "flex-1 min-w-0 text-sm font-medium text-stone-800 truncate", "{name}" }
                                                            }
                                                            code { class: "block text-[10px] font-mono text-stone-400 truncate", "{id}" }
                                                            div { class: "flex items-center gap-2",
                                                                button {
                                                                    class: crate::ui::kit::PILL_GHOST,
                                                                    onclick: {
                                                                        let id = id.clone();
                                                                        move |_| crate::infrastructure::platform::open_url(&sheet_url(&id))
                                                                    },
                                                                    {t(&lang, "connections-open")}
                                                                }
                                                                button {
                                                                    class: crate::ui::kit::PILL_GHOST,
                                                                    disabled: tabs_loading().is_some(),
                                                                    onclick: {
                                                                        let id = id.clone();
                                                                        move |_| {
                                                                            if tabs_loading().is_some() { return; }
                                                                            let id = id.clone();
                                                                            tabs_loading.set(Some(id.clone()));
                                                                            status.set(None);
                                                                            spawn(async move {
                                                                                match load_tab_picker(&db(), &id).await {
                                                                                    Ok(picker) => tab_picker.set(Some(picker)),
                                                                                    Err(e) => status.set(Some(e)),
                                                                                }
                                                                                tabs_loading.set(None);
                                                                            });
                                                                        }
                                                                    },
                                                                    if tabs_loading().as_deref() == Some(id.as_str()) {
                                                                        {t(&lang, "connections-tabs-reading")}
                                                                    } else {
                                                                        {t(&lang, "connections-tabs")}
                                                                    }
                                                                }
                                                                button {
                                                                    class: crate::ui::kit::PILL_GHOST,
                                                                    onclick: {
                                                                        let id = id.clone();
                                                                        move |_| {
                                                                            match crate::application::connector_module::unbind_spreadsheet(&db(), &id) {
                                                                                Ok(()) => {
                                                                                    bindings.set(crate::application::connector_module::current_bindings(&db()));
                                                                                    if tab_picker().is_some_and(|(pid, _, _)| pid == id) {
                                                                                        tab_picker.set(None);
                                                                                    }
                                                                                    status.set(None);
                                                                                }
                                                                                Err(e) => status.set(Some(e)),
                                                                            }
                                                                        }
                                                                    },
                                                                    {t(&lang, "connections-disarm")}
                                                                }
                                                            }
                                                            // The armed scope: every tab, or the pinned subset.
                                                            p { class: "text-[10px] text-stone-400 truncate",
                                                                {
                                                                    let pins = pinned_tabs(&db(), &id);
                                                                    if pins.is_empty() { t(&lang, "connections-tabs-all") } else { pins.join(" · ") }
                                                                }
                                                            }
                                                            if tab_picker().is_some_and(|(pid, _, _)| pid == id) {
                                                                TabPinEditor {
                                                                    picker: tab_picker,
                                                                    bindings,
                                                                    status,
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                            }

                                            div { class: "pt-1 border-t border-stone-100 space-y-2",
                                                button {
                                                    class: crate::ui::kit::PILL_GHOST,
                                                    disabled: listing(),
                                                    onclick: move |_| {
                                                        if listing() { return; }
                                                        listing.set(true);
                                                        status.set(None);
                                                        spawn(async move {
                                                            match crate::application::connector_module::arm_list_spreadsheets(&db()).await {
                                                                Ok(list) => sheets.set(list),
                                                                Err(e) => status.set(Some(e)),
                                                            }
                                                            listing.set(false);
                                                        });
                                                    },
                                                    if listing() {
                                                        span { class: "inline-block w-3 h-3 mr-2 border-2 border-stone-400 border-t-transparent rounded-full animate-spin" }
                                                        {t(&lang, "connections-reading")}
                                                    } else if bindings().is_empty() {
                                                        {t(&lang, "connections-choose-sheet")}
                                                    } else {
                                                        {t(&lang, "connections-add-sheet")}
                                                    }
                                                }

                                                if !sheets().is_empty() {
                                                    div { class: "rounded-xl bg-warm-white border border-stone-200 divide-y divide-stone-100 overflow-hidden",
                                                        for s in sheets() {
                                                            div {
                                                                key: "{s.id}",
                                                                class: "flex items-stretch",
                                                                style: "animation: fadeInUp 0.18s ease-out;",
                                                                button {
                                                                    class: "flex-1 min-w-0 text-left px-3 py-2 min-h-[44px] flex items-center gap-2 active:bg-stone-50 transition-colors",
                                                                    onclick: {
                                                                        let id = s.id.clone();
                                                                        let name = s.name.clone();
                                                                        move |_| {
                                                                            let id = id.clone();
                                                                            let name = name.clone();
                                                                            spawn(async move {
                                                                                match crate::application::connector_module::bind_spreadsheet(&db(), &id, &name).await {
                                                                                    Ok(()) => {
                                                                                        bindings.set(crate::application::connector_module::current_bindings(&db()));
                                                                                        status.set(None);
                                                                                        // Arming proposes the tabs (default: all checked).
                                                                                        // Unreachable metadata keeps the whole workbook armed.
                                                                                        if let Ok(picker) = load_tab_picker(&db(), &id).await {
                                                                                            tab_picker.set(Some(picker));
                                                                                        }
                                                                                    }
                                                                                    Err(e) => status.set(Some(e)),
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    if bindings().iter().any(|(bid, _)| bid == &s.id) {
                                                                        span { class: "text-ios-green shrink-0 text-xs", "✓" }
                                                                    }
                                                                    div { class: "min-w-0",
                                                                        p { class: "text-xs text-stone-700 truncate", "{s.name}" }
                                                                        if !s.modified_at.is_empty() {
                                                                            p { class: "text-[10px] text-stone-400 truncate", {short_date(&s.modified_at)} }
                                                                        }
                                                                    }
                                                                }
                                                                button {
                                                                    class: "shrink-0 px-3 flex items-center text-[11px] text-stone-400 border-l border-stone-100 active:bg-stone-50 transition-colors",
                                                                    onclick: {
                                                                        let id = s.id.clone();
                                                                        move |_| crate::infrastructure::platform::open_url(&sheet_url(&id))
                                                                    },
                                                                    {t(&lang, "connections-open")}
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn sheet_url(id: &str) -> String {
    format!("https://docs.google.com/spreadsheets/d/{id}/edit")
}

// ISO 8601 -> "YYYY-MM-DD HH:MM" so two same-named sheets are told apart. ASCII, so byte slicing is safe.
fn short_date(iso: &str) -> String {
    if iso.len() >= 16 {
        format!("{} {}", &iso[..10], &iso[11..16])
    } else {
        iso.to_string()
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
    crate::infrastructure::platform::open_url(&auth_url);

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
